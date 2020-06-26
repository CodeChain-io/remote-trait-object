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
use super::export_import::*;
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

impl<T: ?Sized + Service + ExportServiceArc<T>> Serialize for SArc<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer, {
        let service = self.take();
        let handle = T::export(port_thread_local::get_port(), service);
        handle.serialize(serializer)
    }
}

impl<'de, T: ?Sized + Service + ImportServiceArc<T>> Deserialize<'de> for SArc<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>, {
        let handle = HandleToExchange::deserialize(deserializer)?;
        Ok(SArc::new(T::import(port_thread_local::get_port(), handle)))
    }
}

#[cfg(test)]
mod tests {
    mod mock {
        use super::super::port_thread_local;
        use crate::port::null_weak_port;

        pub fn set_global_port() {
            port_thread_local::set_port(null_weak_port());
        }
    }

    mod serialize_test {
        use super::super::SArc;
        use super::mock;
        use crate::{ExportServiceArc, HandleToExchange, Port, Service};
        use std::sync::{Arc, Weak};

        trait Foo: Service {
            fn get_handle_to_exchange(&self) -> HandleToExchange;
        }

        struct FooImpl {
            pub handle_to_exchange: HandleToExchange,
        }
        impl FooImpl {
            pub fn new(handle: u32) -> Self {
                Self {
                    handle_to_exchange: HandleToExchange(handle),
                }
            }
        }
        impl Foo for FooImpl {
            fn get_handle_to_exchange(&self) -> HandleToExchange {
                self.handle_to_exchange
            }
        }
        impl Service for FooImpl {}

        impl ExportServiceArc<dyn Foo> for dyn Foo {
            fn export(_port: Weak<dyn Port>, object: Arc<dyn Foo>) -> HandleToExchange {
                object.get_handle_to_exchange()
            }
        }

        /// This test checks SArc<dyn Test> is serialized as HandleToExchange or not
        #[test]
        fn test_serialize() {
            mock::set_global_port();

            {
                let foo_arc: Arc<dyn Foo> = Arc::new(FooImpl::new(3));
                let foo_sarc = SArc::new(foo_arc.clone());
                let bytes = serde_cbor::to_vec(&foo_sarc).unwrap();
                let handle_to_exchange: HandleToExchange = serde_cbor::from_slice(&bytes).unwrap();
                assert_eq!(handle_to_exchange.0, 3);
            }

            {
                let foo_arc: Arc<dyn Foo> = Arc::new(FooImpl::new(23));
                let foo_sarc = SArc::new(foo_arc.clone());
                let bytes = serde_cbor::to_vec(&foo_sarc).unwrap();
                let handle_to_exchange: HandleToExchange = serde_cbor::from_slice(&bytes).unwrap();
                assert_eq!(handle_to_exchange.0, 23);
            }
        }
    }

    mod deserialize_test {
        use super::super::SArc;
        use super::mock;
        use crate::{HandleToExchange, ImportServiceArc, Port, Service};
        use std::sync::{Arc, Weak};

        trait Foo: Service {
            fn get_handle_to_exchange(&self) -> HandleToExchange;
        }
        struct FooImpl {
            handle_to_exchange: HandleToExchange,
        }
        impl Foo for FooImpl {
            fn get_handle_to_exchange(&self) -> HandleToExchange {
                self.handle_to_exchange
            }
        }
        impl Service for FooImpl {}
        impl ImportServiceArc<dyn Foo> for dyn Foo {
            fn import(_port: Weak<dyn Port>, handle: HandleToExchange) -> Arc<dyn Foo> {
                Arc::new(FooImpl {
                    handle_to_exchange: handle,
                })
            }
        }

        #[test]
        fn test_deserialize() {
            mock::set_global_port();

            {
                let handle_to_exchange = HandleToExchange(32);
                let serialized_handle = serde_cbor::to_vec(&handle_to_exchange).unwrap();
                let dyn_foo: SArc<dyn Foo> = serde_cbor::from_slice(&serialized_handle).unwrap();
                assert_eq!(dyn_foo.unwrap().get_handle_to_exchange().0, 32);
            }

            {
                let handle_to_exchange = HandleToExchange(2);
                let serialized_handle = serde_cbor::to_vec(&handle_to_exchange).unwrap();
                let dyn_foo: SArc<dyn Foo> = serde_cbor::from_slice(&serialized_handle).unwrap();
                assert_eq!(dyn_foo.unwrap().get_handle_to_exchange().0, 2);
            }
        }
    }
}
