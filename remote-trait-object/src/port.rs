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
use crate::service::*;
use client::Client;
use std::sync::Arc;

pub trait Port: Send + Sync + 'static {
    fn call(&self, arg: &[u8]) -> Vec<u8>;
    fn delete_request(&self, id: ServiceObjectId);
    /// TODO: Assign id automatically and return it.
    fn register(&self, id: String, handle_to_register: Box<dyn Service>);
}

pub struct BasicPort {
    registry: Arc<ServiceForwarder>,
    client: Client,
}

impl Port for BasicPort {
    fn call(&self, arg: &[u8]) -> Vec<u8> {
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

    pub fn get_registry(&self) -> Arc<ServiceForwarder> {
        self.registry.clone()
    }
}
