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

use crate::ipc::multiplex::{self, ForwardResult, MultiplexResult, Multiplexer};
use crate::ipc::{IpcRecv, IpcSend};
use crate::packet::{PacketView, SlotType};
use crate::port::{client::Client, server::Server, BasicPort, Port};
use std::sync::{Arc, Weak};

pub struct Context {
    multiplexer: Option<Multiplexer>,
    server: Option<Server>,
    port: Option<Arc<BasicPort>>,
}

impl Context {
    pub fn new<S: IpcSend + 'static, R: IpcRecv + 'static>(ipc_send: S, ipc_recv: R) -> Self {
        let MultiplexResult {
            multiplexer,
            request_recv,
            response_recv,
            multiplexed_send,
        } = Multiplexer::multiplex::<R, S, PacketForward>(ipc_send, ipc_recv);
        let client = Client::new(multiplexed_send.clone(), response_recv);
        let port = BasicPort::new(client);
        let server = Server::new(port.get_registry(), multiplexed_send, request_recv);

        Context {
            multiplexer: Some(multiplexer),
            server: Some(server),
            port: Some(port),
        }
    }

    pub fn get_port(&self) -> Weak<dyn Port> {
        Arc::downgrade(&self.port.clone().expect("It becomes None only when the context is dropped.")) as Weak<dyn Port>
    }

    pub fn disable_garbage_collection(&self) {
        self.port.as_ref().expect("It becomes None only when the context is dropped.").set_no_drop();
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        self.multiplexer.take().expect("It becomes None only when the context is dropped.").shutdown();
        // Shutdown server after multiplexer
        self.server.take().expect("It becomes None only when the context is dropped.").shutdown();
        // Shutdown port after multiplexer
        Arc::try_unwrap(self.port.take().expect("It becomes None only when the context is dropped."))
            .unwrap()
            .shutdown();
    }
}

pub struct PacketForward;

impl multiplex::Forward for PacketForward {
    fn forward(packet: PacketView) -> ForwardResult {
        match packet.slot().get_type() {
            SlotType::Request => ForwardResult::Request,
            SlotType::Response => ForwardResult::Response,
        }
    }
}
