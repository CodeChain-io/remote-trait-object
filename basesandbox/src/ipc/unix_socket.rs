use super::*;
use crossbeam::channel::{self, Receiver, Sender};
use mio::net::{UnixListener, UnixStream};
use mio::{Events, Interest, Poll, Token};
use parking_lot::Mutex;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

const POLL_TOKEN: Token = Token(0);

// TODO: Separate 8 bytes size prefix, which is for packet framing, out of pure streaming implementation
// and let it stay as just one of instance of the communication protocol.

fn send_routine(
    queue: Receiver<Vec<u8>>,
    write_signal: Receiver<Result<(), ()>>,
    socket: Arc<Mutex<UnixStream>>,
) -> Result<(), String> {
    #[derive(Debug)]
    enum Error {
        ExpectedTermination,
        UnexpectedError(String),
    }

    let send_helper = |buf: &[u8]| {
        let mut sent = 0;
        while sent < buf.len() {
            // NOTE: Never replace r directly into match.
            // If then, the mutex will be locked during the whole match statement!
            let r = socket.lock().write(&buf[sent..]);
            match r {
                Ok(x) => sent += x,
                Err(e) => {
                    match e.kind() {
                        std::io::ErrorKind::UnexpectedEof => return Err(Error::ExpectedTermination),
                        std::io::ErrorKind::WouldBlock => write_signal // spurious wakeup
                            .recv()
                            .map_err(|x| Error::UnexpectedError(format!("Write signal doesn't arrive: {}", x)))?
                            .map_err(|_| Error::ExpectedTermination)?,
                        _ => panic!(e),
                    }
                }
            }
        }
        assert_eq!(sent, buf.len());
        Ok(())
    };

    loop {
        let x = match queue.recv() {
            Ok(x) => x,
            Err(_) => return Ok(()),
        };
        if x.is_empty() {
            return Ok(())
        }
        let size: [u8; 8] = x.len().to_be_bytes();
        match send_helper(&size) {
            Ok(_) => (),
            Err(Error::ExpectedTermination) => return Ok(()),
            Err(Error::UnexpectedError(s)) => return Err(s),
        }
        match send_helper(&x) {
            Ok(_) => (),
            Err(Error::ExpectedTermination) => return Ok(()),
            Err(Error::UnexpectedError(s)) => return Err(s),
        }
    }
}

fn recv_routine(
    queue: Sender<Vec<u8>>,
    read_signal: Receiver<Result<(), ()>>,
    socket: Arc<Mutex<UnixStream>>,
) -> Result<(), String> {
    #[derive(Debug)]
    enum Error {
        ExpectedTermination,
        UnexpectedError(String),
    }

    let recv_helper = |buf: &mut [u8]| {
        let mut read = 0;
        while read < buf.len() {
            // NOTE: Never replace r directly into match.
            // If then, the mutex will be locked during the whole match statement!
            let r = socket.lock().read(&mut buf[read..]);
            match r {
                Ok(x) => {
                    if x == 0 {
                        return Err(Error::ExpectedTermination)
                    } else {
                        read += x
                    }
                }
                Err(e) => match e.kind() {
                    std::io::ErrorKind::UnexpectedEof => return Err(Error::ExpectedTermination),
                    std::io::ErrorKind::WouldBlock => read_signal
                        .recv()
                        .map_err(|x| Error::UnexpectedError(format!("Read signal doesn't arrive: {}", x)))?
                        .map_err(|_| Error::ExpectedTermination)?, // spurious wakeup
                    e => panic!(e),
                },
            }
        }
        assert_eq!(read, buf.len());
        Ok(())
    };
    loop {
        let mut size_buf = [0 as u8; 8];
        match recv_helper(&mut size_buf) {
            Ok(_) => (),
            Err(Error::ExpectedTermination) => return Ok(()),
            Err(Error::UnexpectedError(s)) => return Err(s),
        }
        let size = usize::from_be_bytes(size_buf);

        assert_ne!(size, 0);
        let mut result: Vec<u8> = vec![0; size];
        match recv_helper(&mut result) {
            Ok(_) => (),
            Err(Error::ExpectedTermination) => return Ok(()),
            Err(Error::UnexpectedError(s)) => return Err(s),
        }
        if queue.send(result).is_err() {
            return Ok(())
        }
    }
}

fn poll_routine(
    exit_flag: Arc<AtomicBool>,
    write_signal: Sender<Result<(), ()>>,
    recv_signal: Sender<Result<(), ()>>,
    mut poll: Poll,
) {
    let mut events = Events::with_capacity(100);
    loop {
        if let Err(e) = poll.poll(&mut events, None) {
            if e.kind() != std::io::ErrorKind::Interrupted {
                // interrupt frequently happens while debugging.
                panic!(e);
            }
        }
        for event in events.iter() {
            assert_eq!(events.iter().next().unwrap().token(), POLL_TOKEN, "Invalid socket event");

            // If it is going to exit, it's ok to fail to send signal (some spurious signals come)
            let exit = if exit_flag.load(Ordering::Relaxed) {
                Ok(())
            } else {
                Err(())
            };

            // termintation
            if event.is_write_closed() || event.is_read_closed() {
                // It might fail depending on the scheduling, but is not a problem.
                let _ = write_signal.send(Err(()));
                let _ = recv_signal.send(Err(()));
                return
            }
            if event.is_writable() {
                // ditto.
                let _ = write_signal.send(Ok(()));
            }
            if event.is_readable() {
                exit.or_else(|_| recv_signal.send(Ok(()))).unwrap();
            }
        }
    }
}

struct SocketInternal {
    _send_thread: Option<thread::JoinHandle<()>>,
    _recv_thread: Option<thread::JoinHandle<()>>,
    _poll_thread: Option<thread::JoinHandle<()>>,
    exit_flag: Arc<AtomicBool>,
    socket: Arc<Mutex<UnixStream>>,
}

impl Drop for SocketInternal {
    fn drop(&mut self) {
        self.exit_flag.store(true, Ordering::Relaxed);
        if let Err(e) = self.socket.lock().shutdown(std::net::Shutdown::Read) {
            assert_eq!(e.kind(), std::io::ErrorKind::NotConnected);
        }
        self._send_thread.take().unwrap().join().unwrap();
        self._recv_thread.take().unwrap().join().unwrap();
        self._poll_thread.take().unwrap().join().unwrap();
    }
}

fn create(mut socket: UnixStream) -> (DomainSocketSend, DomainSocketRecv) {
    let poll = Poll::new().unwrap();
    poll.registry().register(&mut socket, POLL_TOKEN, Interest::WRITABLE.add(Interest::READABLE)).unwrap();

    let socket = Arc::new(Mutex::new(socket));
    // TODO: Choose an appropriate capacities for these channels
    let (send_queue_send, send_queue_recv) = channel::unbounded();
    let (recv_queue_send, recv_queue_recv) = channel::unbounded();
    let (write_signal_send, write_signal_recv) = channel::unbounded();
    let (read_signal_send, read_signal_recv) = channel::unbounded();

    let socket_ = Arc::clone(&socket);
    let _send_thread = Some(
        thread::Builder::new()
            .name("domain_socket_send".to_string())
            .spawn(|| {
                send_routine(send_queue_recv, write_signal_recv, socket_).unwrap();
            })
            .unwrap(),
    );
    let socket_ = Arc::clone(&socket);
    let _recv_thread = Some(
        thread::Builder::new()
            .name("domain_socket_recv".to_string())
            .spawn(|| {
                let terminate_send = recv_queue_send.clone();
                recv_routine(recv_queue_send, read_signal_recv, socket_).unwrap();
                // It might fail depending on the scheduling, but is not a problem.
                let _ = terminate_send.send(Vec::new());
            })
            .unwrap(),
    );

    let exit_flag = Arc::new(AtomicBool::new(false));
    let exit_flag_ = Arc::clone(&exit_flag);
    let _poll_thread = Some(
        thread::Builder::new()
            .name("domain_socket_poll".to_string())
            .spawn(|| poll_routine(exit_flag_, write_signal_send, read_signal_send, poll))
            .unwrap(),
    );

    let socket_internal = Arc::new(SocketInternal {
        _send_thread,
        _recv_thread,
        _poll_thread,
        exit_flag,
        socket,
    });

    (
        DomainSocketSend {
            queue: send_queue_send,
            _socket: Arc::clone(&socket_internal),
        },
        DomainSocketRecv {
            queue: recv_queue_recv,
            socket: socket_internal,
        },
    )
}

pub struct DomainSocketSend {
    queue: Sender<Vec<u8>>,
    _socket: Arc<SocketInternal>,
}

impl IpcSend for DomainSocketSend {
    fn send(&self, data: &[u8]) {
        self.queue.send(data.to_vec()).unwrap();
    }
}

pub struct DomainSocketRecv {
    queue: Receiver<Vec<u8>>,
    socket: Arc<SocketInternal>,
}

impl IpcRecv for DomainSocketRecv {
    type Terminator = Terminator;

    /// Note that DomainSocketRecv is !Sync, so this is guaranteed to be mutual exclusive.
    fn recv(&self, timeout: Option<std::time::Duration>) -> Result<Vec<u8>, RecvError> {
        let x = if let Some(t) = timeout {
            self.queue.recv_timeout(t).map_err(|_| RecvError::TimeOut)?
        } else {
            self.queue.recv().unwrap()
        };
        if x.is_empty() {
            return Err(RecvError::Termination)
        }
        Ok(x)
    }

    fn create_terminator(&self) -> Self::Terminator {
        Terminator {
            socket: Arc::clone(&self.socket),
        }
    }
}

pub struct Terminator {
    socket: Arc<SocketInternal>,
}

impl Terminate for Terminator {
    fn terminate(&self) {
        self.socket.exit_flag.store(true, Ordering::Relaxed);
        if let Err(e) = self.socket.socket.lock().shutdown(std::net::Shutdown::Read) {
            assert_eq!(e.kind(), std::io::ErrorKind::NotConnected);
        }
    }
}

pub struct DomainSocket {
    send: DomainSocketSend,
    recv: DomainSocketRecv,
}

impl IpcSend for DomainSocket {
    fn send(&self, data: &[u8]) {
        self.send.send(data)
    }
}

impl IpcRecv for DomainSocket {
    type Terminator = Terminator;

    fn recv(&self, timeout: Option<std::time::Duration>) -> Result<Vec<u8>, RecvError> {
        self.recv.recv(timeout)
    }

    fn create_terminator(&self) -> Self::Terminator {
        self.recv.create_terminator()
    }
}

impl Ipc for DomainSocket {
    fn arguments_for_both_ends() -> (Vec<u8>, Vec<u8>) {
        let path_gen = || format!("{}/{}", std::env::temp_dir().to_str().unwrap(), generate_random_name());
        let address = path_gen();
        (serde_cbor::to_vec(&(true, &address)).unwrap(), serde_cbor::to_vec(&(false, &address)).unwrap())
    }

    type SendHalf = DomainSocketSend;
    type RecvHalf = DomainSocketRecv;

    fn new(data: Vec<u8>) -> Self {
        let (am_i_server, address): (bool, String) = serde_cbor::from_slice(&data).unwrap();

        // We use spinning for the connection establishment
        let stream = if am_i_server {
            let listener = UnixListener::bind(&address).unwrap();
            (|| {
                for _ in 0..100 {
                    if let Ok(stream) = listener.accept() {
                        return stream
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                panic!("Failed to establish domain socket within a timeout")
            })()
            .0
        } else {
            (|| {
                for _ in 0..100 {
                    if let Ok(stream) = UnixStream::connect(&address) {
                        return stream
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                panic!("Failed to establish domain socket within a timeout")
            })()
        };

        let (send, recv) = create(stream);

        DomainSocket {
            send,
            recv,
        }
    }

    fn split(self) -> (Self::SendHalf, Self::RecvHalf) {
        (self.send, self.recv)
    }
}
