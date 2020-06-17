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

use serde::{Deserialize, Serialize};

pub type InstanceId = u32;
pub type MethodId = u32;

/// This struct represents an index to a service object in port server's registry
#[derive(PartialEq, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ServiceObjectId {
    pub(crate) index: InstanceId,
}

/// Exporter sides's interface to the service object. This will be implemented
/// by each service trait's unique wrapper in the macro
pub trait Dispatch: Send + Sync {
    fn dispatch_and_call(&self, method: MethodId, args: &[u8]) -> Vec<u8>;
}

impl<F> Dispatch for F
where
    F: Fn(MethodId, &[u8]) -> Vec<u8> + Send + Sync,
{
    fn dispatch_and_call(&self, method: MethodId, args: &[u8]) -> Vec<u8> {
        self(method, args)
    }
}

pub trait Service: Send + Sync {
    // TODO: add fn get_port(&self) -> Weak<dyn Port>;
}
