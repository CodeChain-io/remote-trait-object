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

use super::Dispatch;
use super::*;
use std::sync::Arc;

pub struct ServiceToRegister {
    pub(crate) raw: Arc<dyn Dispatch>,
}

impl ServiceToRegister {
    pub fn new(raw: Arc<dyn Dispatch>) -> Self {
        Self {
            raw,
        }
    }
}

// These traits are associated with some specific service trait.
// These tratis will be implement by `dyn ServiceTrait` where `T = dyn ServiceTrait` as well.
// Macro will implement this trait with the target(expanding) service trait.

/// Unused T is for avoiding violation of the orphan rule
/// T will be local type for the crate, and that makes it possible to
/// ```ignore
/// impl IntoService<dyn MyService> for Arc<dyn MyService>
/// ```
pub trait IntoService<T: ?Sized + Service> {
    fn into_service(self) -> ServiceToRegister;
}

/// Unused T is for avoiding violation of the orphan rule, like `IntoService`
pub trait ImportRemote<T: ?Sized + Service>: Sized {
    fn import_remote(port: Weak<dyn Port>, handle: HandleToExchange) -> Self;
}

// These functions are utilities for the generic traits above

pub fn export_service<T: ?Sized + Service>(
    context: &crate::context::Context,
    service: impl IntoService<T>,
) -> HandleToExchange {
    context.get_port().upgrade().unwrap().register(service.into_service().raw)
}

pub fn import_service<T: ?Sized + Service, P: ImportRemote<T>>(
    context: &crate::context::Context,
    handle: HandleToExchange,
) -> P {
    P::import_remote(context.get_port(), handle)
}
