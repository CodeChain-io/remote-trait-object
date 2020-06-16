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
use crossbeam::channel::{Receiver, Sender};

pub struct Client {
    ipc_send: Sender<Packet>,
    ipc_recv: Receiver<Packet>,
}

impl Client {
    pub fn new(ipc_send: Sender<Packet>, ipc_recv: Receiver<Packet>) -> Self {
        Client {
            ipc_send,
            ipc_recv,
        }
    }

    pub fn call(&self, packet: PacketView) -> Packet {
        // Please set call slot
        let mut packet = packet.to_owned();
        packet.set_slot(SlotId::new_request());
        self.ipc_send.send(packet).unwrap();
        self.ipc_recv.recv().unwrap()
    }
}
