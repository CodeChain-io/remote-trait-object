use crate::packet::PacketView;
use crate::transport::{Terminate, TransportError, TransportRecv};
use crate::Config;
use crossbeam::channel::{self, Receiver, Sender};
use parking_lot::Mutex;
use std::thread;

pub struct MultiplexedRecv {
    recv: Receiver<Result<Vec<u8>, TransportError>>,
}

impl TransportRecv for MultiplexedRecv {
    fn recv(&self, timeout: Option<std::time::Duration>) -> Result<Vec<u8>, TransportError> {
        if let Some(timeout) = timeout {
            self.recv.recv_timeout(timeout).unwrap()
        } else {
            self.recv.recv().unwrap()
        }
    }

    fn create_terminator(&self) -> Box<dyn Terminate> {
        unreachable!()
    }
}

#[derive(Debug)]
pub enum ForwardResult {
    Request,
    Response,
}

pub trait Forward {
    fn forward(data: PacketView) -> ForwardResult;
}

pub struct MultiplexResult {
    pub request_recv: MultiplexedRecv,
    pub response_recv: MultiplexedRecv,
    pub multiplexer: Multiplexer,
}

pub struct Multiplexer {
    receiver_thread: Option<thread::JoinHandle<()>>,
    /// Here Mutex is used to make the Multiplxer Sync, while dyn Terminate isn't.
    receiver_terminator: Option<Mutex<Box<dyn Terminate>>>,
}

impl Multiplexer {
    pub fn multiplex<TransportReceiver, Forwarder>(
        config: Config,
        transport_recv: TransportReceiver,
    ) -> MultiplexResult
    where
        TransportReceiver: TransportRecv + 'static,
        Forwarder: Forward,
    {
        let (request_send, request_recv) = channel::bounded(1);
        let (response_send, response_recv) = channel::bounded(1);
        let receiver_terminator: Option<Mutex<Box<dyn Terminate>>> =
            Some(Mutex::new(transport_recv.create_terminator()));

        let receiver_thread = thread::Builder::new()
            .name(format!("[{}] receiver multiplexer", config.name))
            .spawn(move || {
                receiver_loop::<Forwarder, TransportReceiver>(
                    transport_recv,
                    request_send,
                    response_send,
                )
            })
            .unwrap();

        MultiplexResult {
            request_recv: MultiplexedRecv { recv: request_recv },
            response_recv: MultiplexedRecv {
                recv: response_recv,
            },
            multiplexer: Multiplexer {
                receiver_thread: Some(receiver_thread),
                receiver_terminator,
            },
        }
    }

    pub fn shutdown(mut self) {
        self.receiver_terminator
            .take()
            .unwrap()
            .into_inner()
            .terminate();
        self.receiver_thread.take().unwrap().join().unwrap();
    }

    pub fn wait(mut self, _timeout: Option<std::time::Duration>) -> Result<(), Self> {
        self.receiver_thread.take().unwrap().join().unwrap();
        Ok(())
    }
}

fn receiver_loop<Forwarder: Forward, Receiver: TransportRecv>(
    transport_recv: Receiver,
    request_send: Sender<Result<Vec<u8>, TransportError>>,
    response_send: Sender<Result<Vec<u8>, TransportError>>,
) {
    loop {
        let message = match transport_recv.recv(None) {
            Err(err) => {
                request_send.send(Err(err.clone())).unwrap();
                response_send.send(Err(err)).unwrap();
                return;
            }
            Ok(data) => data,
        };

        let packet_view = PacketView::new(&message);
        trace!("Receive message in multiplex {}", packet_view);
        let forward_result = Forwarder::forward(packet_view);

        match forward_result {
            ForwardResult::Request => request_send.send(Ok(message)).unwrap(),
            ForwardResult::Response => response_send.send(Ok(message)).unwrap(),
        }
    }
}

impl Drop for Multiplexer {
    fn drop(&mut self) {
        assert!(self.receiver_thread.is_none(), "Please call shutdown");
    }
}
