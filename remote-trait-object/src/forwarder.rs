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

use crate::packet::PacketView;
use crate::port::Handler;
use crate::service::Dispatch;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fmt;

type ServiceId = String;

pub struct ServiceForwarder {
    service_handlers: RwLock<HashMap<ServiceId, Box<dyn Dispatch>>>,
}

impl fmt::Debug for ServiceForwarder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.service_handlers.read().keys()).finish()
    }
}

impl ServiceForwarder {
    pub fn new() -> Self {
        Self {
            service_handlers: Default::default(),
        }
    }

    pub fn add_service(&self, name: ServiceId, service: Box<dyn Dispatch>) {
        let insert_result = self.service_handlers.write().insert(name.clone(), service);
        if insert_result.is_some() {
            panic!("Duplicated service id {}", name);
        }
    }

    pub fn forward_and_call(&self, packet: PacketView) -> Vec<u8> {
        let service_name = packet.service_name();
        let method = packet.method();
        let data = packet.data();
        let handlers = self.service_handlers.read();
        handlers
            .get(&service_name)
            .unwrap_or_else(|| panic!("Fail to find {} from ServiceForwarder", service_name))
            .dispatch_and_call(method, data)
    }
}

impl Default for ServiceForwarder {
    fn default() -> Self {
        Self::new()
    }
}

impl Handler for ServiceForwarder {
    fn handle(&self, input: PacketView) -> Vec<u8> {
        self.forward_and_call(input)
    }
}
