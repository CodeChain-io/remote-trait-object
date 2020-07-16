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

#[macro_use]
extern crate crossbeam;
#[macro_use]
extern crate log;
extern crate remote_trait_object_macro;

mod context;
mod forwarder;
mod packet;
mod port;
mod queue;
mod service;
#[cfg(test)]
mod tests;
pub mod transport;

pub use context::{Config, Context};
pub use service::export_import::{
    export_service_into_handle, import_service_from_handle, ImportRemote, IntoServiceToRegister, ServiceToRegister,
};
pub use service::id::setup_identifiers;
pub use service::serde_support::ServiceRef;
pub use service::{create_null_service, HandleToExchange, NullService, Service};

#[doc(hidden)]
pub mod macro_env {
    pub use super::*;
    pub use port::Port;
    pub use service::id::{IdMap, MethodIdAtomic, ID_ORDERING, MID_REG};
    pub use service::{Dispatch, Handle, MethodId};
}

// Re-export macro
pub use remote_trait_object_macro::*;
