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
mod types;

pub use self::types::Handler;
use crate::forwarder::ServiceForwarder;
use crate::ipc::multiplex::{MultiplexResult, Multiplexer};
use crate::ipc::{IpcRecv, IpcSend};
use crate::service::*;
use client::Client;
use server::Server;
use std::sync::{Arc, Weak};

pub trait Port: Send + Sync + 'static {
    fn call(&self, arg: String) -> String;
    fn delete_request(&self, id: ServiceObjectId);
    /// TODO: Assign id automatically and return it.
    fn register(&self, id: String, handle_to_register: Box<dyn Service>);
}

pub struct BasicPort {
    registry: Arc<ServiceForwarder>,
    client: Client,
}

impl Port for BasicPort {
    fn call(&self, arg: String) -> String {
        self.client.call(arg)
    }

    fn delete_request(&self, _id: ServiceObjectId) {
        unimplemented!()
    }

    fn register(&self, id: String, service: Box<dyn Service>) {
        self.registry.add_service(id, service);
    }
}

impl BasicPort {
    pub fn new(client: Client) -> Self {
        Self {
            registry: Arc::new(ServiceForwarder::new()),
            client,
        }
    }
}

pub struct Context {
    multiplexer: Option<Multiplexer>,
    server: Option<Server>,
    port: Arc<BasicPort>,
}

impl Context {
    pub fn new<S: IpcSend + 'static, R: IpcRecv + 'static>(ipc_send: S, ipc_recv: R) -> Self {
        let MultiplexResult {
            multiplexer,
            request_recv,
            response_recv,
            multiplexed_send,
        } = Multiplexer::multiplex(ipc_send, ipc_recv);
        let client = client::Client::new(multiplexed_send.clone(), response_recv);
        let port = Arc::new(BasicPort::new(client));
        let server = server::Server::new(port.registry.clone(), multiplexed_send, request_recv);

        Context {
            multiplexer: Some(multiplexer),
            server: Some(server),
            port,
        }
    }

    pub fn get_port(&self) -> Weak<dyn Port> {
        Arc::downgrade(&self.port) as Weak<dyn Port>
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // Shutdown multiplexer before server
        self.multiplexer.take().unwrap().shutdown();
        self.server.take().unwrap().shutdown();
    }
}
