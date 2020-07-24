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

use crate::packet::{Packet, PacketView};
use crate::transport::{RecvError, Terminate, TransportRecv};
use crate::Config;
use crossbeam::channel::{self, Receiver, Sender};
use parking_lot::Mutex;
use std::thread;

#[derive(Debug)]
pub enum ForwardResult {
    Request,
    Response,
}

pub trait Forward {
    fn forward(data: PacketView) -> ForwardResult;
}

pub struct MultiplexResult {
    pub request_recv: Receiver<Packet>,
    pub response_recv: Receiver<Packet>,
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
        Forwarder: Forward, {
        let (request_send, request_recv) = channel::bounded(1);
        let (response_send, response_recv) = channel::bounded(1);
        let receiver_terminator: Option<Mutex<Box<dyn Terminate>>> =
            Some(Mutex::new(transport_recv.create_terminator()));

        let receiver_thread = thread::Builder::new()
            .name(format!("[{}] receiver multiplexer", config.name))
            .spawn(move || receiver_loop::<Forwarder, TransportReceiver>(transport_recv, request_send, response_send))
            .unwrap();

        MultiplexResult {
            request_recv,
            response_recv,
            multiplexer: Multiplexer {
                receiver_thread: Some(receiver_thread),
                receiver_terminator,
            },
        }
    }

    pub fn shutdown(mut self) {
        self.receiver_terminator.take().unwrap().into_inner().terminate();
        self.receiver_thread.take().unwrap().join().unwrap();
    }
}

fn receiver_loop<Forwarder: Forward, Receiver: TransportRecv>(
    transport_recv: Receiver,
    request_send: Sender<Packet>,
    response_send: Sender<Packet>,
) {
    loop {
        let message = match transport_recv.recv(None) {
            Err(RecvError::TimeOut) => panic!(),
            Err(RecvError::Termination) => {
                debug!("transport_recv is closed in multiplex");
                return
            }
            Ok(data) => data,
        };

        let packet_view = PacketView::new(&message);
        trace!("Receive message in multiplex {}", packet_view);
        let forward_result = Forwarder::forward(packet_view);
        let packet = Packet::new_from_buffer(message);

        match forward_result {
            ForwardResult::Request => request_send.send(packet).unwrap(),

            ForwardResult::Response => response_send.send(packet).unwrap(),
        }
    }
}

impl Drop for Multiplexer {
    fn drop(&mut self) {
        assert!(self.receiver_thread.is_none(), "Please call shutdown");
    }
}
