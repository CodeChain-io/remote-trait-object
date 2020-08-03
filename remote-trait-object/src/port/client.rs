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

use crate::packet::{Packet, PacketView, SlotId};
use crate::queue::Queue;
use crate::transport::{TransportError, TransportRecv, TransportSend};
use crate::Config;
use crossbeam::channel::RecvTimeoutError::{Disconnected, Timeout};
use crossbeam::channel::{bounded, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time;

/// CallSlot represents an instance of call to the another module
#[derive(Debug)]
struct CallSlot {
    id: SlotId,
    response: Receiver<Result<Packet, TransportError>>,
}

#[derive(Debug)]
pub struct Client {
    config: Config,
    call_slots: Arc<Queue<CallSlot>>,
    transport_send: Arc<dyn TransportSend>,
    receiver_thread: Option<thread::JoinHandle<()>>,
    joined_event_receiver: Receiver<()>,
}

impl Client {
    pub fn new(config: Config, transport_send: Arc<dyn TransportSend>, transport_recv: Box<dyn TransportRecv>) -> Self {
        let (joined_event_sender, joined_event_receiver) = bounded(1);
        let callslot_size = SlotId::new(config.call_slots as u32);
        let call_slots = Arc::new(Queue::new(callslot_size.as_usize()));
        let mut to_slot_receivers = Vec::with_capacity(callslot_size.as_usize());

        for i in 0..callslot_size.as_raw() {
            let (send_to_slot_recv, recv_for_slot) = bounded(1);
            call_slots
                .push(CallSlot {
                    id: SlotId::new(i),
                    response: recv_for_slot,
                })
                .expect("Client does not call close");
            to_slot_receivers.push(send_to_slot_recv);
        }

        let name = config.name.clone();

        Client {
            config,
            call_slots,
            transport_send,
            receiver_thread: Some(
                thread::Builder::new()
                    .name(format!("[{}] client", name))
                    .spawn(move || {
                        receive_loop(transport_recv, to_slot_receivers);
                        joined_event_sender.send(()).unwrap();
                    })
                    .unwrap(),
            ),
            joined_event_receiver,
        }
    }

    pub fn call(&self, packet: PacketView) -> Packet {
        // TODO: handle the error
        let slot = self.call_slots.pop(self.config.call_timeout).expect("Too many calls on port");

        let packet = {
            let mut packet = packet.to_owned();
            packet.set_slot(slot.id.into_request());
            packet
        };

        // TODO: handle the error
        self.transport_send.send(packet.buffer(), self.config.call_timeout).unwrap();
        let response_packet = slot.response.recv().expect(
            "counterparty send is managed by client. \n\
        This error might be due to drop after disconnection of the two remote-trait-object contexts. \n\
        Please consider disable_garbage_collection() or explicit drop for the imported services.",
        );

        self.call_slots.push(slot).expect("Client does not close the queue");

        // TODO: handle the error
        response_packet.unwrap()
    }

    pub fn shutdown(&mut self) {
        match self.joined_event_receiver.recv_timeout(time::Duration::from_millis(100)) {
            Err(Timeout) => {
                panic!(
                    "There may be a deadlock or misuse of Client. Call Client::shutdown after Multiplexer::shutdown"
                );
            }
            Err(Disconnected) => {
                panic!("Maybe receive_loop thread panics");
            }
            Ok(_) => {}
        }
        self.receiver_thread.take().unwrap().join().unwrap();
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        assert!(self.receiver_thread.is_none(), "Please call shutdown");
    }
}

fn receive_loop(
    transport_recv: Box<dyn TransportRecv>,
    to_slot_receivers: Vec<Sender<Result<Packet, TransportError>>>,
) {
    loop {
        match transport_recv.recv(None) {
            Ok(x) => {
                let packet = Packet::new_from_buffer(x);
                let slot_id = packet.view().slot();
                to_slot_receivers[slot_id.as_usize()]
                    .send(Ok(packet))
                    .expect("Slot receivers are managed in Client. Client must be dropped after this thread");
            }
            Err(TransportError::Termination) => return,
            Err(_err) => {
                // TODO: Broadcast the error to all **active** call slots
                return
            }
        };
    }
}
