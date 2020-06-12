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
use crossbeam::channel::RecvTimeoutError::{Disconnected, Timeout};
use crossbeam::channel::{self, Receiver, Sender};
use std::thread;
use std::time;

pub struct Server {
    receiver_thread: Option<thread::JoinHandle<()>>,
    joined_event_receiver: Receiver<()>,
}

impl Server {
    pub fn new<H>(handler: H, ipc_send: Sender<String>, ipc_recv: Receiver<String>) -> Self
    where
        H: Handler + Send + 'static, {
        let (joined_event_sender, joined_event_receiver) = channel::bounded(1);
        let receiver_thread = thread::Builder::new()
            .name("port server receiver".into())
            .spawn(move || {
                receiver(handler, ipc_send, ipc_recv);
                joined_event_sender.send(()).expect("Server will be dropped after thread is joined");
            })
            .unwrap();

        Server {
            receiver_thread: Some(receiver_thread),
            joined_event_receiver,
        }
    }

    pub fn shutdown(mut self) {
        match self.joined_event_receiver.recv_timeout(time::Duration::from_millis(100)) {
            Err(Timeout) => {
                panic!("There may be a deadlock or misuse of Server. Call Server::shutdown when ipc_recv is closed");
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

fn receiver<H>(handler: H, ipc_send: Sender<String>, ipc_recv: Receiver<String>)
where
    H: Handler, {
    loop {
        let request = match ipc_recv.recv() {
            Ok(request) => request,
            Err(_err) => {
                // ipc_recv is closed.
                return
            }
        };
        let response = handler.handle(request);
        ipc_send.send(format!("response:{}", response)).unwrap();
    }
}
