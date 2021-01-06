mod complex_trait;

use crate as remote_trait_object;
use remote_trait_object_macro as rto_macro;

use crate::forwarder::ServiceObjectId;
use crate::packet::{Packet, PacketView};
use crate::port::*;
use crate::raw_exchange::{HandleToExchange, ImportProxy, IntoSkeleton};
use crate::service::*;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

struct TestDispatchMap {
    last_id: u32,
    map: HashMap<u32, Arc<dyn Dispatch>>,
}

impl TestDispatchMap {
    pub fn new() -> Self {
        Self {
            last_id: 0,
            map: HashMap::new(),
        }
    }

    fn insert(&mut self, service_object: Arc<dyn Dispatch>) -> u32 {
        self.last_id += 1;
        self.map.insert(self.last_id, service_object);
        self.last_id
    }

    fn get_cloned(&mut self, id: u32) -> Arc<dyn Dispatch> {
        Arc::clone(&self.map.get(&id).unwrap())
    }

    fn remove(&mut self, id: u32) {
        self.map.remove(&id);
    }

    fn len(&self) -> usize {
        self.map.len()
    }
}

pub(crate) struct TestPort {
    dispatch_map: Mutex<TestDispatchMap>,
}

impl TestPort {
    pub fn new() -> Self {
        Self {
            dispatch_map: Mutex::new(TestDispatchMap::new()),
        }
    }

    fn register_len(&self) -> usize {
        self.dispatch_map.lock().len()
    }
}

impl std::fmt::Debug for TestPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

impl Port for TestPort {
    fn call(&self, packet: PacketView) -> Packet {
        let object_id = packet.object_id();
        let dispatcher = self.dispatch_map.lock().get_cloned(object_id);
        let response = dispatcher.dispatch_and_call(packet.method(), packet.data());
        let mut response_packet = Packet::new_response_from_request(packet);
        response_packet.append_data(&response);
        response_packet
    }

    fn delete_request(&self, id: ServiceObjectId) {
        self.dispatch_map.lock().remove(id);
    }

    fn register_service(&self, service_object: Arc<dyn Dispatch>) -> HandleToExchange {
        HandleToExchange(self.dispatch_map.lock().insert(service_object))
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
            (
                format!("{}_{}_{}", s1, "None", self.mul),
                "ByeBye".to_owned(),
            )
        }
    }
}

#[test]
fn macro1() {
    let port = Arc::new(TestPort::new());
    let port_weak = Arc::downgrade(&port);

    let object = Box::new(MyObject { mul: 4 }) as Box<dyn Service1>;
    let handle = port.register_service(object.into_skeleton().raw);
    let proxy = <Box<dyn Service1> as ImportProxy<dyn Service1>>::import_proxy(port_weak, handle);

    assert_eq!(
        proxy.f1(1, &2, &[3, 4], (5, 6), &(7, "8".to_owned())),
        (1 + 2 + 3 + 4 + 5 + 6 + 7 + 8) * 4
    );
    assert_eq!(
        proxy.f2("Hello", &Some(123)),
        ("Hello_123_4".to_owned(), "Bye".to_owned())
    );
    assert_eq!(
        proxy.f2("Hello", &None),
        ("Hello_None_4".to_owned(), "ByeBye".to_owned())
    );
    drop(proxy);
    assert_eq!(port.register_len(), 0);
}

#[rto_macro::service]
trait Hello: Service {
    fn f(&self, v: &[(i32, i32)]) -> i32;
}

struct SimpleHello;
impl Service for SimpleHello {}

impl Hello for SimpleHello {
    fn f(&self, v: &[(i32, i32)]) -> i32 {
        v.iter().map(|(x, y)| x + y).sum()
    }
}

/// This trait causes a compile error without `no_skeleton`
#[rto_macro::service(no_skeleton)]
trait HelloWithRef: Service {
    fn f(&self, v: &[&(&i32, &i32)]) -> i32;
}

#[test]
fn macro_no_skeleton() {
    let port = Arc::new(TestPort::new());
    let port_weak = Arc::downgrade(&port);

    let object = Box::new(SimpleHello) as Box<dyn Hello>;

    let handle = port.register_service(object.into_skeleton().raw);
    let proxy =
        <Box<dyn HelloWithRef> as ImportProxy<dyn HelloWithRef>>::import_proxy(port_weak, handle);

    let source = vec![1, 2, 3, 4];
    let source2 = vec![(&source[0], &source[1]), (&source[2], &source[3])];
    let source3 = vec![&source2[0], &source2[1]];

    assert_eq!(proxy.f(&source3), 10);
    drop(proxy);
    assert_eq!(port.register_len(), 0);
}
