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
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;

pub type ServiceObjectId = u32;

pub struct ServiceForwarder {
    service_objects: RwLock<HashMap<ServiceObjectId, Arc<dyn Dispatch>>>,
    available_ids: RwLock<VecDeque<ServiceObjectId>>,
}

impl fmt::Debug for ServiceForwarder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.service_objects.read().keys()).finish()
    }
}

impl ServiceForwarder {
    pub fn new() -> Self {
        Self {
            service_objects: Default::default(),
            available_ids: RwLock::new({
                let mut queue = VecDeque::new();
                for i in 0..100 {
                    queue.push_back(i)
                }
                queue
            }),
        }
    }

    pub fn register_service_object(&self, service_object: Arc<dyn Dispatch>) -> ServiceObjectId {
        let id = self.available_ids.write().pop_front().expect("Too many service objects had been created");
        assert!(self.service_objects.write().insert(id, service_object).is_none());
        id
    }

    pub fn forward_and_call(&self, packet: PacketView) -> Vec<u8> {
        let object_id = packet.object_id();
        let method = packet.method();
        let data = packet.data();
        let handlers = self.service_objects.read();
        handlers
            .get(&object_id)
            .unwrap_or_else(|| panic!("Fail to find {} from ServiceForwarder", object_id))
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
