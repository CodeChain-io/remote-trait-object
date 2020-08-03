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

use super::types::Handler;
use crate::packet::Packet;
use crate::transport::{TransportError, TransportRecv, TransportSend};
use crate::Config;
use crossbeam::channel::RecvTimeoutError::{Disconnected, Timeout};
use crossbeam::channel::{self, Receiver};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;
use std::time;

pub struct Server {
    receiver_thread: Option<thread::JoinHandle<()>>,
    joined_event_receiver: Receiver<()>,
}

impl Server {
    pub fn new<H>(
        config: Config,
        handler: Arc<H>,
        transport_send: Arc<dyn TransportSend>,
        transport_recv: Box<dyn TransportRecv>,
    ) -> Self
    where
        H: Handler + Send + 'static, {
        let (joined_event_sender, joined_event_receiver) = channel::bounded(1);
        let receiver_thread = thread::Builder::new()
            .name(format!("[{}] port server receiver", config.name))
            .spawn(move || {
                receiver(config, handler, transport_send, transport_recv);
                joined_event_sender.send(()).expect("Server will be dropped after thread is joined");
            })
            .unwrap();

        Server {
            receiver_thread: Some(receiver_thread),
            joined_event_receiver,
        }
    }

    pub fn shutdown(mut self) {
        match self.joined_event_receiver.recv_timeout(time::Duration::from_millis(500)) {
            Err(Timeout) => {
                panic!(
                    "There may be a deadlock or misuse of Server. Call Server::shutdown when transport_recv is closed"
                );
            }
            Err(Disconnected) => {
                panic!("Maybe receiver thread panics");
            }
            Ok(_) => {}
        }

        self.receiver_thread.take().unwrap().join().unwrap();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        assert!(self.receiver_thread.is_none(), "Please call shutdown")
    }
}

fn handle_single_call<H: Handler>(
    packet: Packet,
    handler: Arc<H>,
    transport_send: Arc<dyn TransportSend>,
    count: Arc<AtomicI32>,
) {
    let response = handler.handle(packet.view());
    let mut response_packet = Packet::new_response_from_request(packet.view());
    response_packet.append_data(&response);
    if let Err(_err) = transport_send.send(response_packet.buffer(), None) {
        // TODO: report the error to the context
        count.fetch_sub(1, Ordering::Release);
        return
    };
    count.fetch_sub(1, Ordering::Release);
}

fn receiver<H>(
    config: Config,
    handler: Arc<H>,
    transport_send: Arc<dyn TransportSend>,
    transport_recv: Box<dyn TransportRecv>,
) where
    H: Handler + 'static, {
    let count = Arc::new(AtomicI32::new(0));
    loop {
        match transport_recv.recv(None) {
            Ok(request) => {
                let packet = Packet::new_from_buffer(request);
                let handler = Arc::clone(&handler);
                let transport_send = Arc::clone(&transport_send);

                count.fetch_add(1, Ordering::Release);
                let count = Arc::clone(&count);
                config.thread_pool.lock().execute(move || handle_single_call(packet, handler, transport_send, count));
            }
            Err(TransportError::Termination) => break,
            Err(_err) => {
                // TODO: report this error to the context
                break
            }
        }
    }
    // transport_recv is terminated.

    // TODO: handle too many loops
    while count.load(Ordering::Acquire) != 0 {
        thread::sleep(std::time::Duration::from_millis(1));
    }
}
