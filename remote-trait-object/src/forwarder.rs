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
use crate::port::{null_weak_port, Handler, Port};
use crate::raw_exchange::Skeleton;
use crate::service::Dispatch;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Weak};

pub type ServiceObjectId = u32;
pub const DELETE_REQUEST: crate::service::MethodId = std::u32::MAX;
pub const META_SERVICE_OBJECT_ID: ServiceObjectId = 0;
pub const INITIAL_SERVICE_OBJECT_ID: ServiceObjectId = 1;

pub struct ServiceForwarder {
    service_objects: RwLock<HashMap<ServiceObjectId, Arc<dyn Dispatch>>>,
    available_ids: RwLock<VecDeque<ServiceObjectId>>,
    port: RwLock<Weak<dyn Port>>,
}

impl fmt::Debug for ServiceForwarder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.service_objects.read().keys()).finish()
    }
}

impl ServiceForwarder {
    pub fn new(meta_service: Skeleton) -> Self {
        let service_objects: RwLock<HashMap<ServiceObjectId, Arc<dyn Dispatch>>> = Default::default();
        service_objects.write().insert(META_SERVICE_OBJECT_ID, meta_service.raw);
        let mut available_ids = VecDeque::new();
        for i in 0u32..100 {
            if i != META_SERVICE_OBJECT_ID {
                available_ids.push_back(i);
            }
        }

        Self {
            service_objects: Default::default(),
            available_ids: RwLock::new((0u32..100).collect()),
            port: RwLock::new(null_weak_port()),
        }
    }

    pub fn with_initial_service(meta_service: Skeleton, service_object: Skeleton) -> Self {
        let service_objects: RwLock<HashMap<ServiceObjectId, Arc<dyn Dispatch>>> = Default::default();
        service_objects.write().insert(META_SERVICE_OBJECT_ID, meta_service.raw);
        service_objects.write().insert(INITIAL_SERVICE_OBJECT_ID, service_object.raw);
        let mut available_ids = VecDeque::new();
        for i in 0u32..100 {
            if i != META_SERVICE_OBJECT_ID && i != INITIAL_SERVICE_OBJECT_ID {
                available_ids.push_back(i);
            }
        }

        Self {
            service_objects,
            available_ids: RwLock::new(available_ids),
            port: RwLock::new(null_weak_port()),
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

        if method == DELETE_REQUEST {
            self.delete(object_id);
            Vec::new()
        } else {
            crate::service::serde_support::port_thread_local::set_port(self.port.read().clone());
            let handler = Arc::clone(
                self.service_objects
                    .read()
                    .get(&object_id)
                    .unwrap_or_else(|| panic!("Fail to find {} from ServiceForwarder", object_id)),
            );
            let result = handler.dispatch_and_call(method, data);
            crate::service::serde_support::port_thread_local::remove_port();
            result
        }
    }

    pub fn clear(&self) {
        self.service_objects.write().clear();
        // we don't restore available_ids here becuase clear() will be called in termination phase
    }

    fn delete(&self, id: ServiceObjectId) {
        self.service_objects.write().remove(&id).unwrap();
        self.available_ids.write().push_back(id);
    }

    /// Be careful of this circular reference
    pub fn set_port(&self, port: Weak<dyn Port>) {
        *self.port.write() = port
    }
}

impl Handler for ServiceForwarder {
    fn handle(&self, input: PacketView) -> Vec<u8> {
        self.forward_and_call(input)
    }
}
