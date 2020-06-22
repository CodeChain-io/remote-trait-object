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

use crate as remote_trait_object;
use remote_trait_object_macro as rto_macro;

use crate::forwarder::ServiceObjectId;
use crate::packet::{Packet, PacketView};
use crate::port::*;
use crate::service::*;
use parking_lot::RwLock;
use std::sync::Arc;

struct TestPort {
    target: RwLock<Option<Arc<dyn Dispatch>>>,
}

impl std::fmt::Debug for TestPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

impl Port for TestPort {
    fn call(&self, packet: PacketView) -> Packet {
        let response = self.target.read().as_ref().unwrap().dispatch_and_call(packet.method(), packet.data());
        let mut response_packet = Packet::new_response_from_request(packet);
        response_packet.append_data(&response);
        response_packet
    }

    fn delete_request(&self, _id: ServiceObjectId) {
        self.target.write().take().unwrap();
    }

    fn register(&self, service_object: Arc<dyn Dispatch>) -> HandleToExchange {
        self.target.write().replace(service_object);
        HandleToExchange(1234) //doesn't matter
    }
}

#[rto_macro::service]
pub trait Service1: Service {
    fn f1(&self, a1: i32, a2: &i32, a3: &[i32], a4: (i32, i32), a5: &(i32, String)) -> i32;
    fn f2(&self, s1: &str, a2: &Option<i32>) -> (String, String);
}

struct MyObject {
    mul: i32,
}

impl Service for MyObject {}

impl Service1 for MyObject {
    fn f1(&self, a1: i32, a2: &i32, a3: &[i32], a4: (i32, i32), a5: &(i32, String)) -> i32 {
        let sum: i32 = a3.iter().sum();
        (a1 + *a2 + sum + a4.0 + a4.1 + a5.0 + a5.1.parse::<i32>().unwrap()) * self.mul
    }

    fn f2(&self, s1: &str, a2: &Option<i32>) -> (String, String) {
        if let Some(x) = a2.as_ref() {
            (format!("{}_{}_{}", s1, x, self.mul), "Bye".to_owned())
        } else {
            (format!("{}_{}_{}", s1, "None", self.mul), "ByeBye".to_owned())
        }
    }
}

// TODO: Replace manual Remote/Dispatcher construction to import/export
#[test]
fn macro1() {
    let port = Arc::new(TestPort {
        target: Default::default(),
    });
    let port_weak = Arc::downgrade(&port);

    let object = Arc::new(MyObject {
        mul: 4,
    }) as Arc<dyn Service1>;
    let dispatcher = Arc::new(Service1Dispatcher {
        object,
    }) as Arc<dyn Dispatch>;
    let handle = port.register(dispatcher);
    let remote = Service1Remote {
        handle: Handle {
            port: port_weak,
            id: handle.0,
        },
    };

    assert_eq!(remote.f1(1, &2, &[3, 4], (5, 6), &(7, "8".to_owned())), (1 + 2 + 3 + 4 + 5 + 6 + 7 + 8) * 4);
    assert_eq!(remote.f2("Hello", &Some(123)), ("Hello_123_4".to_owned(), "Bye".to_owned()));
    assert_eq!(remote.f2("Hello", &None), ("Hello_None_4".to_owned(), "ByeBye".to_owned()));
    drop(remote);
    assert!(port.target.read().is_none());
}
