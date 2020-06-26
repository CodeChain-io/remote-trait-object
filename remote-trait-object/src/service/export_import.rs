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

use super::*;
use parking_lot::RwLock;

// These traits are associated with some specific service trait.
// These tratis will be implement by `dyn ServiceTrait` where `T = dyn ServiceTrait` as well.
// Macro will implement this trait with the target(expanding) service trait.

pub trait ExportServiceBox<T: ?Sized + Service> {
    fn export(port: Weak<dyn Port>, object: Box<T>) -> HandleToExchange;
}

pub trait ExportServiceArc<T: ?Sized + Service> {
    fn export(port: Weak<dyn Port>, object: Arc<T>) -> HandleToExchange;
}

pub trait ExportServiceRwLock<T: ?Sized + Service> {
    fn export(port: Weak<dyn Port>, object: Arc<RwLock<T>>) -> HandleToExchange;
}

pub trait ImportServiceBox<T: ?Sized + Service> {
    fn import(port: Weak<dyn Port>, handle: HandleToExchange) -> Box<T>;
}

pub trait ImportServiceArc<T: ?Sized + Service> {
    fn import(port: Weak<dyn Port>, handle: HandleToExchange) -> Arc<T>;
}

pub trait ImportServiceRwLock<T: ?Sized + Service> {
    fn import(port: Weak<dyn Port>, handle: HandleToExchange) -> Arc<RwLock<T>>;
}

// These functions are utilities for the generic traits above

pub fn export_service_box<T: ?Sized + Service + ExportServiceBox<T>>(
    context: &crate::context::Context,
    service: Box<T>,
) -> HandleToExchange {
    <T as ExportServiceBox<T>>::export(context.get_port(), service)
}

pub fn export_service_arc<T: ?Sized + Service + ExportServiceArc<T>>(
    context: &crate::context::Context,
    service: Arc<T>,
) -> HandleToExchange {
    <T as ExportServiceArc<T>>::export(context.get_port(), service)
}

pub fn export_service_rwlock<T: ?Sized + Service + ExportServiceRwLock<T>>(
    context: &crate::context::Context,
    service: Arc<RwLock<T>>,
) -> HandleToExchange {
    <T as ExportServiceRwLock<T>>::export(context.get_port(), service)
}

pub fn import_service_box<T: ?Sized + Service + ImportServiceBox<T>>(
    context: &crate::context::Context,
    handle: HandleToExchange,
) -> Box<T> {
    <T as ImportServiceBox<T>>::import(context.get_port(), handle)
}

pub fn import_service_arc<T: ?Sized + Service + ImportServiceArc<T>>(
    context: &crate::context::Context,
    handle: HandleToExchange,
) -> Arc<T> {
    <T as ImportServiceArc<T>>::import(context.get_port(), handle)
}

pub fn import_service_rwlock<T: ?Sized + Service + ImportServiceRwLock<T>>(
    context: &crate::context::Context,
    handle: HandleToExchange,
) -> Arc<RwLock<T>> {
    <T as ImportServiceRwLock<T>>::import(context.get_port(), handle)
}
