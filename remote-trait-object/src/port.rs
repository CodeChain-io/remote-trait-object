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
use crate::forwarder::{ServiceObjectId, DELETE_REQUEST};
use crate::packet::{Packet, PacketView};
use crate::raw_exchange::Skeleton;
use crate::service::*;
use client::Client;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Weak,
};

pub trait Port: std::fmt::Debug + Send + Sync + 'static {
    fn call(&self, packet: PacketView) -> Packet;
    fn delete_request(&self, id: ServiceObjectId);
    fn register_service(&self, service_object: Arc<dyn Dispatch>) -> HandleToExchange;
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
    /// If this is on, the port will not request delete
    /// This is useful when the port-port connection is terminating and you don't really
    /// care about the garabage collection.
    no_drop: AtomicBool,
}

impl Port for BasicPort {
    fn call(&self, packet: PacketView) -> Packet {
        self.client.as_ref().unwrap().call(packet)
    }

    fn delete_request(&self, id: ServiceObjectId) {
        if self.no_drop.load(Ordering::SeqCst) {
            return
        }
        let packet = Packet::new_request(id, DELETE_REQUEST, &[]);
        assert!(self.client.as_ref().unwrap().call(packet.view()).data().is_empty());
    }

    fn register_service(&self, service_object: Arc<dyn Dispatch>) -> HandleToExchange {
        HandleToExchange(self.registry.register_service_object(service_object))
    }
}

impl BasicPort {
    pub fn new(client: Client, meta_sevice: Skeleton) -> Arc<Self> {
        let arc = Arc::new(Self {
            registry: Arc::new(ServiceForwarder::new(meta_sevice)),
            client: Some(client),
            no_drop: AtomicBool::new(false),
        });
        let arc2 = arc.clone() as Arc<dyn Port>;
        arc.registry.set_port(Arc::downgrade(&arc2));
        arc
    }

    pub fn with_initial_service(client: Client, meta_sevice: Skeleton, initial_service: Skeleton) -> Arc<Self> {
        let arc = Arc::new(Self {
            registry: Arc::new(ServiceForwarder::with_initial_service(meta_sevice, initial_service)),
            client: Some(client),
            no_drop: AtomicBool::new(false),
        });
        let arc2 = arc.clone() as Arc<dyn Port>;
        arc.registry.set_port(Arc::downgrade(&arc2));
        arc
    }

    pub fn get_registry(&self) -> Arc<ServiceForwarder> {
        self.registry.clone()
    }

    pub fn clear_registry(&self) {
        self.registry.clear();
    }

    /// Please call shutdown after Multiplexer::shutdown
    pub fn shutdown(mut self) {
        self.client.take().unwrap().shutdown();
    }

    pub fn set_no_drop(&self) {
        self.no_drop.store(true, Ordering::SeqCst);
    }
}

impl Drop for BasicPort {
    fn drop(&mut self) {
        assert!(self.client.is_none(), "Please call shutdown");
    }
}
