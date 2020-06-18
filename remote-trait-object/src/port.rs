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

pub mod client;
pub mod server;
pub mod types;

pub use self::types::Handler;
use crate::forwarder::ServiceForwarder;
use crate::forwarder::ServiceObjectId;
use crate::packet::{Packet, PacketView};
use crate::service::*;
use client::Client;
use std::sync::{Arc, Weak};

pub trait Port: Send + Sync + 'static {
    fn call(&self, packet: PacketView) -> Packet;
    fn delete_request(&self, id: ServiceObjectId);
    fn register(&self, service_object: Arc<dyn Dispatch>) -> HandleToExchange;
}

/// Weak::new() is not implemented for ?Sized.
/// See https://github.com/rust-lang/rust/issues/50513
pub fn null_weak_port() -> Weak<dyn Port> {
    Weak::<BasicPort>::new() as Weak<dyn Port>
}

#[derive(Debug)]
pub struct BasicPort {
    registry: Arc<ServiceForwarder>,
    /// client is None only in the drop function.
    client: Option<Client>,
}

impl Port for BasicPort {
    fn call(&self, packet: PacketView) -> Packet {
        self.client.as_ref().unwrap().call(packet)
    }

    fn delete_request(&self, id: ServiceObjectId) {
        debug!("Delete requested: {}", id);
        // TODO: Delete it actually
    }

    fn register(&self, service_object: Arc<dyn Dispatch>) -> HandleToExchange {
        HandleToExchange(self.registry.register_service_object(service_object))
    }
}

impl BasicPort {
    pub fn new(client: Client) -> Arc<Self> {
        let arc = Arc::new(Self {
            registry: Arc::new(ServiceForwarder::new()),
            client: Some(client),
        });
        let arc2 = arc.clone() as Arc<dyn Port>;
        arc.registry.set_port(Arc::downgrade(&arc2));
        arc
    }

    pub fn get_registry(&self) -> Arc<ServiceForwarder> {
        self.registry.clone()
    }

    /// Please call shutdown after Multiplexer::shutdown
    pub fn shutdown(mut self) {
        self.client.take().unwrap().shutdown();
    }
}

impl Drop for BasicPort {
    fn drop(&mut self) {
        assert!(self.client.is_none(), "Please call shutdown");
    }
}
