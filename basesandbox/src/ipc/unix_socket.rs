use super::*;
use mio::net::{UnixListener, UnixStream};
use mio::{Events, Interest, Poll, Token};
use parking_lot::Mutex;
use std::io::{Read, Write};
use std::sync::Arc;
const POLL_TOKEN: Token = Token(0);
use crossbeam::channel::{self, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

fn send_routine(
    queue: Receiver<Vec<u8>>,
    write_signal: Receiver<Result<(), ()>>,
    socket: Arc<Mutex<UnixStream>>,
) -> Result<(), ()> {
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
                        // spurious wakeup
                        std::io::ErrorKind::UnexpectedEof => return Err(()),
                        std::io::ErrorKind::WouldBlock => write_signal.recv().unwrap().map_err(|_| ())?,
                        _ => panic!(e),
                    }
                }
            }
        }
        assert_eq!(sent, buf.len());
        Ok(())
    };

    loop {
        let x = queue.recv().map_err(|_| ())?;
        if x.is_empty() {
            return Err(())
        }
        let size: [u8; 8] = x.len().to_be_bytes();
        send_helper(&size)?;
        send_helper(&x)?;
    }
}

fn recv_routine(
    queue: Sender<Vec<u8>>,
    read_signal: Receiver<Result<(), ()>>,
    socket: Arc<Mutex<UnixStream>>,
) -> Result<(), ()> {
    let recv_helper = |buf: &mut [u8]| {
        let mut read = 0;
        while read < buf.len() {
            // NOTE: Never replace r directly into match.
            // If then, the mutex will be locked during the whole match statement!
            let r = socket.lock().read(&mut buf[read..]);
            match r {
                Ok(x) => {
                    if x == 0 {
                        return Err(())
                    } else {
                        read += x
                    }
                }
                Err(e) => match e.kind() {
                    std::io::ErrorKind::UnexpectedEof => return Err(()),
                    std::io::ErrorKind::WouldBlock => read_signal.recv().unwrap().map_err(|_| ())?, // spurious wakeup
                    e => panic!(e),
                },
            }
        }
        assert_eq!(read, buf.len());
        Ok(())
    };
    loop {
        let mut size_buf = [0 as u8; 8];
        recv_helper(&mut size_buf)?;
        let size = usize::from_be_bytes(size_buf);

        assert_ne!(size, 0);
        let mut result: Vec<u8> = vec![0; size];
        recv_helper(&mut result)?;
        queue.send(result).map_err(|_| ())?;
    }
}

fn poll_routine(
    exit_flag: Arc<AtomicBool>,
    write_signal: Sender<Result<(), ()>>,
    recv_signal: Sender<Result<(), ()>>,
    mut poll: Poll,
) {
    // TODO: does the capacity matter if it's 1?
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
                write_signal.send(Err(())).ok();
                recv_signal.send(Err(())).ok();
                return
            }
            if event.is_writable() {
                // we don't really care if it succeeds, especially in the termination phase.
                write_signal.send(Ok(())).ok();
            }
            if event.is_readable() {
                // ditto.
                recv_signal.send(Ok(())).map_err(|_| ()).or(exit).unwrap();
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
    let (send_queue_send, send_queue_recv) = channel::unbounded();
    let (recv_queue_send, recv_queue_recv) = channel::unbounded();
    let (write_signal_send, write_signal_recv) = channel::unbounded();
    let (read_signal_send, read_signal_recv) = channel::unbounded();

    let socket_ = socket.clone();
    let _send_thread = Some(thread::spawn(|| {
        send_routine(send_queue_recv, write_signal_recv, socket_).unwrap_err();
    }));
    let socket_ = socket.clone();
    let _recv_thread = Some(thread::spawn(|| {
        let terminate_send = recv_queue_send.clone();
        recv_routine(recv_queue_send, read_signal_recv, socket_).unwrap_err();
        terminate_send.send(Vec::new()).ok(); // doesn't matter even if it fails
    }));

    let exit_flag = Arc::new(AtomicBool::new(false));
    let exit_flag_ = exit_flag.clone();
    let _poll_thread = Some(thread::spawn(|| poll_routine(exit_flag_, write_signal_send, read_signal_send, poll)));

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
            _socket: socket_internal.clone(),
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
        Terminator(self.socket.clone())
    }
}

pub struct Terminator(Arc<SocketInternal>);

impl Terminate for Terminator {
    fn terminate(&self) {
        self.0.exit_flag.store(true, Ordering::Relaxed);
        if let Err(e) = (self.0).socket.lock().shutdown(std::net::Shutdown::Read) {
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
