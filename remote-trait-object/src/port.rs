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

mod client;
mod server;

use crate::ipc::multiplex::{MultiplexResult, Multiplexer};
use std::sync::mpsc::{Receiver, Sender};

pub struct Port {
    _multiplexer: Multiplexer,
    _server: server::Server,
    client: client::Client,
}

impl Port {
    pub fn new<F>(send: Sender<String>, recv: Receiver<String>, dispatcher: F) -> Self
    where
        F: Fn(String) -> String + Send + 'static,
    {
        let MultiplexResult {
            multiplexer,
            request_recv,
            response_recv,
        } = Multiplexer::multiplex(recv);
        let client = client::Client::new(send.clone(), response_recv);
        let _server = server::Server::new(dispatcher, send, request_recv);
        Self {
            client,
            _server,
            _multiplexer: multiplexer,
        }
    }

    pub fn call(&self, message: String) -> String {
        self.client.call(message)
    }
}
