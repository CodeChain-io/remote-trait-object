// Copyright 2020 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::ipc::{IpcRecv, IpcSend, RecvError, Terminate};
use crossbeam::channel::{self, Receiver, Sender};
use parking_lot::Mutex;
use std::thread;

pub struct MultiplexResult {
    pub request_recv: Receiver<Vec<u8>>,
    pub response_recv: Receiver<Vec<u8>>,
    pub multiplexed_send: Sender<Vec<u8>>,
    pub multiplexer: Multiplexer,
}

pub struct Multiplexer {
    receiver_thread: Option<thread::JoinHandle<()>>,
    /// Here Mutex is used to make the Multiplxer Sync, while dyn Terminate isn't.
    receiver_terminator: Option<Mutex<Box<dyn Terminate>>>,
    sender_thread: Option<thread::JoinHandle<()>>,
    sender_terminator: Sender<()>,
}

impl Multiplexer {
    pub fn multiplex<IpcReceiver, IpcSender>(ipc_send: IpcSender, ipc_recv: IpcReceiver) -> MultiplexResult
    where
        IpcReceiver: IpcRecv + 'static,
        IpcSender: IpcSend + 'static, {
        let (request_send, request_recv) = channel::bounded(1);
        let (response_send, response_recv) = channel::bounded(1);
        let receiver_terminator: Option<Mutex<Box<dyn Terminate>>> =
            Some(Mutex::new(Box::new(ipc_recv.create_terminator())));

        let receiver_thread = thread::Builder::new()
            .name("receiver multiplexer".into())
            .spawn(move || receiver_loop(ipc_recv, request_send, response_send))
            .unwrap();

        let (multiplexed_send, from_multiplexed_send) = channel::bounded(1);
        let (sender_terminator, recv_sender_terminate) = channel::bounded(1);
        let sender_thread = thread::Builder::new()
            .name("sender multiplexer".into())
            .spawn(move || sender_loop(ipc_send, from_multiplexed_send, recv_sender_terminate))
            .unwrap();

        MultiplexResult {
            request_recv,
            response_recv,
            multiplexed_send,
            multiplexer: Multiplexer {
                receiver_thread: Some(receiver_thread),
                sender_thread: Some(sender_thread),
                receiver_terminator,
                sender_terminator,
            },
        }
    }

    pub fn shutdown(mut self) {
        self.receiver_terminator.take().unwrap().into_inner().terminate();
        self.receiver_thread.take().unwrap().join().unwrap();
        if let Err(_err) = self.sender_terminator.send(()) {
            debug!("Sender thread is dropped before shutdown multiplexer");
        }
        self.sender_thread.take().unwrap().join().unwrap();
    }
}

fn receiver_loop(ipc_recv: impl IpcRecv, request_send: Sender<Vec<u8>>, response_send: Sender<Vec<u8>>) {
    loop {
        let original_message = match ipc_recv.recv(None) {
            Err(RecvError::TimeOut) => panic!(),
            Err(RecvError::Termination) => {
                debug!("ipc_recv is closed in multiplex");
                return
            }
            Ok(data) => data,
        };

        // FIXME: parsing is not the role of the Multiplexer.
        let message = parse(original_message.clone());
        match message {
            Some(ParseResult::Request(request)) => request_send.send(request).unwrap(),
            Some(ParseResult::Response(response)) => response_send.send(response).unwrap(),
            None => {
                panic!("Receved invalid message {:?}", original_message);
            }
        }
    }
}

fn sender_loop(ipc_sender: impl IpcSend, from_multiplexed_send: Receiver<Vec<u8>>, from_terminator: Receiver<()>) {
    loop {
        let data = select! {
            recv(from_multiplexed_send) -> msg => match msg {
                Ok(data) => data,
                Err(_) => {
                    debug!("All multiplexed send is closed");
                    return;
                }
            },
            recv(from_terminator) -> msg => match msg {
                Ok(()) => {
                    // Received termination flag
                    return;
                }
                Err(err) => {
                    panic!("Multiplexer is dropped before sender thread {}", err);
                }
            },
        };
        ipc_sender.send(&data);
    }
}

impl Drop for Multiplexer {
    fn drop(&mut self) {
        assert!(self.receiver_thread.is_none(), "Please call shutdown");
    }
}

enum ParseResult {
    Request(Vec<u8>),
    Response(Vec<u8>),
}

fn parse(message: Vec<u8>) -> Option<ParseResult> {
    // FIXME
    let message = String::from_utf8(message).unwrap();
    let request_prefix = "request:";
    let response_prefix = "response:";
    if message.starts_with(request_prefix) {
        // FIXME
        Some(ParseResult::Request(message.trim_start_matches(request_prefix).as_bytes().to_vec()))
    } else if message.starts_with(response_prefix) {
        // FIXME
        Some(ParseResult::Response(message.trim_start_matches(response_prefix).as_bytes().to_vec()))
    } else {
        None
    }
}
