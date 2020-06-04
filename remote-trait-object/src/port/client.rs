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

use std::sync::mpsc::{Receiver, Sender};

pub struct Client {
    ipc_send: Sender<String>,
    ipc_recv: Receiver<String>,
}

impl Client {
    pub fn new(ipc_send: Sender<String>, ipc_recv: Receiver<String>) -> Self {
        Client { ipc_send, ipc_recv }
    }

    pub fn call(&self, msg: String) -> String {
        self.ipc_send.send(format!("request:{}", msg)).unwrap();
        self.ipc_recv.recv().unwrap()
    }
}
