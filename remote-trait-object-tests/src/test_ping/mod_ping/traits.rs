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

use remote_trait_object::*;
use std::sync::{Arc, Weak};

#[rto_macro::service]
pub trait Ping: Service {
    fn ping(&self) -> String;
}

/// TODO: All below things will be generated by the macro

pub struct PingHandler {
    object: Arc<dyn Ping>,
}

impl PingHandler {
    pub fn new(object: Arc<dyn Ping>) -> Self {
        Self {
            object,
        }
    }
}

impl Dispatch for PingHandler {
    fn dispatch_and_call(&self, method: MethodId, args: &[u8]) -> Vec<u8> {
        trace!("Ping received {}({:?}) request", method, args);
        if method == 70 {
            serde_cbor::to_vec(&self.object.ping()).unwrap()
        } else {
            panic!("Dispatch failed: {}({:?})", method, args)
        }
    }
}

impl ExportService<dyn Ping> for dyn Ping {
    fn export(port: Weak<dyn Port>, object: Arc<dyn Ping>) -> HandleToExchange {
        port.upgrade().unwrap().register(Arc::new(PingDispatcher::new(object)))
    }
}

impl ImportService<dyn Ping> for dyn Ping {
    fn import(port: Weak<dyn Port>, handle: HandleToExchange) -> Arc<dyn Ping> {
        Arc::new(PingRemote {
            handle: Handle::careful_new(handle, port),
        })
    }
}
