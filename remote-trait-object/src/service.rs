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

/// This struct represents an index to a service object in port server's registry
#[derive(PartialEq, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct ServiceObjectId {
    pub(crate) index: InstanceId,
}

pub trait Dispatch {
    fn dispatch_and_call(&self, input: &[u8]) -> Vec<u8> {
        input.to_vec()
    }
}

pub trait Service: Dispatch + Send + Sync {}
