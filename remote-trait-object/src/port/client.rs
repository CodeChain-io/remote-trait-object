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

use crossbeam::channel::{Receiver, Sender};

pub struct Client {
    ipc_send: Sender<Vec<u8>>,
    ipc_recv: Receiver<Vec<u8>>,
}

impl Client {
    pub fn new(ipc_send: Sender<Vec<u8>>, ipc_recv: Receiver<Vec<u8>>) -> Self {
        Client {
            ipc_send,
            ipc_recv,
        }
    }

    pub fn call(&self, msg: &[u8]) -> Vec<u8> {
        // FIXME
        let msg = String::from_utf8(msg.to_vec()).unwrap();
        self.ipc_send.send(format!("request:{}", msg).as_bytes().to_vec()).unwrap();
        // Need call slots to find the exact response
        self.ipc_recv.recv().unwrap()
    }
}
