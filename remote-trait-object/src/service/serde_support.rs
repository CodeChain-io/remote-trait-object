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
use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

pub struct SArc<T: ?Sized + Service> {
    value: std::cell::Cell<Option<Arc<T>>>,
}

impl<T: ?Sized + Service> SArc<T> {
    pub fn new(value: Arc<T>) -> Self {
        SArc {
            value: std::cell::Cell::new(Some(value)),
        }
    }

    pub(crate) fn take(&self) -> Arc<T> {
        self.value.take().unwrap()
    }

    pub fn unwrap(self) -> Arc<T> {
        self.value.take().unwrap()
    }
}

/// This manages thread-local keys for port, which will be used in serialization of
/// SArc. Note that this is required even in the inter-process setup.
/// TODO: check that serde doens't spawn a thread while serializing.
pub mod port_thread_local {
    use super::*;
    use std::cell::RefCell;

    // TODO
    // If a service call another service, this PORT setting might be stacked (at most twice).
    // We check that the consistency of stacking for an assertion purpose.
    // However it might be costly considering the frequency of this operations,
    // so please replace this with unchecking logic
    // after the library becomes stabilized.
    thread_local!(static PORT: RefCell<Vec<Weak<dyn Port>>> = RefCell::new(Vec::new()));

    pub fn set_port(port: Weak<dyn Port>) {
        PORT.with(|k| {
            k.try_borrow_mut().unwrap().push(port);
            assert!(k.borrow().len() <= 2);
        })
    }

    pub fn get_port() -> Weak<dyn Port> {
        PORT.with(|k| k.borrow().last().unwrap().clone())
    }

    pub fn remove_port() {
        PORT.with(|k| {
            k.try_borrow_mut().unwrap().pop().unwrap();
        })
    }
}

impl<T: ?Sized + Service + ExportService<T>> Serialize for SArc<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer, {
        let service = self.take();
        let handle = T::export(port_thread_local::get_port(), service);
        handle.serialize(serializer)
    }
}

impl<'de, T: ?Sized + Service + ImportService<T>> Deserialize<'de> for SArc<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>, {
        let handle = HandleToExchange::deserialize(deserializer)?;
        Ok(SArc::new(T::import(port_thread_local::get_port(), handle)))
    }
}
