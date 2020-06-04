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
use std::thread;

pub struct Server {
    _receiver_thread: Option<thread::JoinHandle<()>>,
}

impl Server {
    pub fn new<F>(dispatcher: F, ipc_send: Sender<String>, ipc_recv: Receiver<String>) -> Self
    where
        F: Fn(String) -> String + Send + 'static,
    {
        let receiver_thread = thread::Builder::new()
            .name("port server receiver".into())
            .spawn(move || {
                receiver(dispatcher, ipc_send, ipc_recv);
            })
            .unwrap();

        Server {
            _receiver_thread: Some(receiver_thread),
        }
    }
}

fn receiver<F>(dispatcher: F, ipc_send: Sender<String>, ipc_recv: Receiver<String>)
where
    F: Fn(String) -> String,
{
    loop {
        let request = ipc_recv.recv().unwrap();
        let response = dispatcher(request);
        ipc_send.send(format!("response:{}", response)).unwrap();
    }
}
