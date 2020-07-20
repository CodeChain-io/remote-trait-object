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

use super::export_import::*;
use super::*;
use crate::{raw_exchange::HandleToExchange, service::Dispatch};
use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use std::marker::PhantomData;
use std::sync::Arc;

enum ExportOrImport {
    Export(Arc<dyn Dispatch>),
    Import(HandleToExchange, Weak<dyn Port>),
}

pub struct ServiceRef<T: ?Sized + Service> {
    service: ExportOrImport,
    _marker: PhantomData<T>,
}

impl<T: ?Sized + Service> ServiceRef<T> {
    pub fn from_service(service: impl IntoServiceToRegister<T>) -> Self {
        Self {
            service: ExportOrImport::Export(service.into_service_to_register().raw),
            _marker: PhantomData,
        }
    }

    pub fn into_remote<P: ImportRemote<T>>(self) -> P {
        match self.service {
            ExportOrImport::Import(handle, port) => P::import_remote(port, handle),
            _ => panic!("You must call import() on an imported ServiceRef"),
        }
    }
}

/// This manages thread-local pointer of the port, which will be used in serialization of
/// service objects wrapped in the S* pointers. Cuttently it is the only way to deliver the port
/// within the de/serialization context.
/// TODO: check that serde doens't spawn a thread while serializing.
pub(crate) mod port_thread_local {
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

impl<T: ?Sized + Service> Serialize for ServiceRef<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer, {
        let error = "You must not de/serialize ServiceRef by yourself. If you not, this is a bug.";
        if let ExportOrImport::Export(service) = &self.service {
            debug_assert_eq!(Arc::strong_count(service), 1);
            let handle = port_thread_local::get_port().upgrade().expect(error).register(Arc::clone(service));
            handle.serialize(serializer)
        } else {
            panic!(error)
        }
    }
}

impl<'de, T: ?Sized + Service> Deserialize<'de> for ServiceRef<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>, {
        let handle = HandleToExchange::deserialize(deserializer)?;
        Ok(ServiceRef {
            service: ExportOrImport::Import(handle, port_thread_local::get_port()),
            _marker: std::marker::PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    mod serialize_test {
        use super::super::ServiceRef;
        use crate::macro_env::*;
        use crate::packet::*;
        use crate::port::Port;
        use crate::service::ServiceObjectId;
        use crate::*;
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::{Arc, Weak};

        #[derive(Debug)]
        pub struct MockPort {
            count: AtomicU32,
        }

        impl Port for MockPort {
            fn call(&self, _packet: PacketView) -> Packet {
                unimplemented!()
            }

            fn register(&self, _service_object: Arc<dyn Dispatch>) -> HandleToExchange {
                self.count.fetch_add(1, Ordering::SeqCst);
                HandleToExchange(123)
            }

            fn delete_request(&self, _id: ServiceObjectId) {
                unimplemented!()
            }
        }

        trait Foo: Service {}

        struct FooImpl;
        impl Foo for FooImpl {}
        impl Service for FooImpl {}
        impl Dispatch for FooImpl {
            fn dispatch_and_call(&self, _method: MethodId, _args: &[u8]) -> Vec<u8> {
                unimplemented!()
            }
        }

        impl IntoServiceToRegister<dyn Foo> for Arc<dyn Foo> {
            fn into_service_to_register(self) -> crate::macro_env::ServiceToRegister {
                crate::macro_env::create_service_to_register(Arc::new(FooImpl))
            }
        }

        /// This test checks SArc<dyn Test> is serialized as HandleToExchange or not
        #[test]
        fn test_serialize() {
            let port = Arc::new(MockPort {
                count: AtomicU32::new(0),
            });
            let weak_port = Arc::downgrade(&port) as Weak<dyn Port>;
            super::super::port_thread_local::set_port(weak_port);

            {
                let foo_arc: Arc<dyn Foo> = Arc::new(FooImpl);
                let foo_sarc = ServiceRef::from_service(foo_arc.clone());
                let bytes = serde_cbor::to_vec(&foo_sarc).unwrap();
                let handle_to_exchange: HandleToExchange = serde_cbor::from_slice(&bytes).unwrap();
                assert_eq!(handle_to_exchange.0, 123);
                assert_eq!(port.count.load(Ordering::SeqCst), 1);
            }

            {
                let foo_arc: Arc<dyn Foo> = Arc::new(FooImpl);
                let foo_sarc = ServiceRef::from_service(foo_arc.clone());
                let bytes = serde_cbor::to_vec(&foo_sarc).unwrap();
                let handle_to_exchange: HandleToExchange = serde_cbor::from_slice(&bytes).unwrap();
                assert_eq!(handle_to_exchange.0, 123);
                assert_eq!(port.count.load(Ordering::SeqCst), 2);
            }
        }
    }

    mod deserialize_test {
        use super::super::ServiceRef;
        use crate::port::Port;
        use crate::{raw_exchange::*, Service};
        use std::sync::Weak;

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
        impl ImportRemote<dyn Foo> for Box<dyn Foo> {
            fn import_remote(_port: Weak<dyn Port>, handle: HandleToExchange) -> Box<dyn Foo> {
                Box::new(FooImpl {
                    handle_to_exchange: handle,
                })
            }
        }

        #[test]
        fn test_deserialize() {
            super::super::port_thread_local::set_port(crate::port::null_weak_port());

            {
                let handle_to_exchange = HandleToExchange(32);
                let serialized_handle = serde_cbor::to_vec(&handle_to_exchange).unwrap();
                let dyn_foo: ServiceRef<dyn Foo> = serde_cbor::from_slice(&serialized_handle).unwrap();
                assert_eq!(dyn_foo.into_remote::<Box<dyn Foo>>().get_handle_to_exchange().0, 32);
            }

            {
                let handle_to_exchange = HandleToExchange(2);
                let serialized_handle = serde_cbor::to_vec(&handle_to_exchange).unwrap();
                let dyn_foo: ServiceRef<dyn Foo> = serde_cbor::from_slice(&serialized_handle).unwrap();
                assert_eq!(dyn_foo.into_remote::<Box<dyn Foo>>().get_handle_to_exchange().0, 2);
            }
        }
    }
}
