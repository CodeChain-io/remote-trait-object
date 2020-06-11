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
use crate::ipc::{IpcRecv, IpcSend};

pub struct Port {
    multiplexer: Option<Multiplexer>,
    server: Option<server::Server>,
    client: client::Client,
}

impl Port {
    pub fn new<F, IpcSender, IpcReceiver>(ipc_send: IpcSender, ipc_recv: IpcReceiver, dispatcher: F) -> Self
    where
        F: Fn(String) -> String + Send + 'static,
        IpcSender: IpcSend + 'static,
        IpcReceiver: IpcRecv + 'static, {
        let MultiplexResult {
            multiplexer,
            request_recv,
            response_recv,
            multiplexed_send,
        } = Multiplexer::multiplex(ipc_send, ipc_recv);
        let client = client::Client::new(multiplexed_send.clone(), response_recv);
        let server = server::Server::new(dispatcher, multiplexed_send, request_recv);
        Self {
            client,
            server: Some(server),
            multiplexer: Some(multiplexer),
        }
    }

    pub fn call(&self, message: String) -> String {
        self.client.call(message)
    }
}

impl Drop for Port {
    fn drop(&mut self) {
        // Shutdown multiplexer before server
        self.multiplexer.take().unwrap().shutdown();
        self.server.take().unwrap().shutdown();
    }
}
