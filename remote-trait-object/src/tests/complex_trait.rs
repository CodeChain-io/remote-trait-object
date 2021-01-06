use super::TestPort;
use crate as remote_trait_object;
use crate::port::Port;
use crate::raw_exchange::*;
use crate::{Service, ServiceRef, ServiceToExport};
use remote_trait_object_macro as rto_macro;
use std::sync::{Arc, Mutex};

#[rto_macro::service]
trait A: Service {
    fn service_object_as_argument(&self, b: ServiceRef<dyn B>);
    fn service_object_as_return(&self) -> ServiceRef<dyn B>;
    fn recursive_service_object(&self) -> ServiceRef<dyn A>;
    fn get_recursion_count(&self) -> u32;
}

#[rto_macro::service]
trait B: Service {
    fn inc(&self);
    fn get(&self) -> i32;
}

struct SimpleA {
    recursion_count: u32,
}

impl SimpleA {
    pub fn new() -> Self {
        Self { recursion_count: 0 }
    }

    pub fn with_recursion_count(recursion_count: u32) -> Self {
        Self { recursion_count }
    }
}

impl A for SimpleA {
    fn service_object_as_argument(&self, b: ServiceRef<dyn B>) {
        let b: Box<dyn B> = b.unwrap_import().into_proxy();
        assert_eq!(0, b.get());
        b.inc();
        b.inc();
        b.inc();
        assert_eq!(3, b.get());
    }

    fn service_object_as_return(&self) -> ServiceRef<dyn B> {
        let b = Box::new(SimpleB::new()) as Box<dyn B>;
        ServiceRef::Export(ServiceToExport::new(b))
    }

    fn recursive_service_object(&self) -> ServiceRef<dyn A> {
        let a = Box::new(SimpleA::with_recursion_count(self.recursion_count + 1)) as Box<dyn A>;
        ServiceRef::Export(ServiceToExport::new(a))
    }

    fn get_recursion_count(&self) -> u32 {
        self.recursion_count
    }
}

impl Service for SimpleA {}

struct SimpleB {
    count: Mutex<i32>,
}

impl SimpleB {
    pub fn new() -> Self {
        Self {
            count: Mutex::new(0),
        }
    }
}

impl Service for SimpleB {}
impl B for SimpleB {
    fn inc(&self) {
        *self.count.lock().unwrap() += 1
    }
    fn get(&self) -> i32 {
        *self.count.lock().unwrap()
    }
}

fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

fn create_proxy_a(port: Arc<dyn Port>) -> Arc<dyn A> {
    let a: Arc<dyn A> = Arc::new(SimpleA::new());
    let handle = port.register_service(a.into_skeleton().raw);
    ImportProxy::import_proxy(Arc::downgrade(&port), handle)
}

#[test]
fn service_object_as_return() {
    init_logger();

    let port = Arc::new(TestPort::new());
    let proxy_a = create_proxy_a(port.clone());

    let proxy_b: Box<dyn B> = proxy_a
        .service_object_as_return()
        .unwrap_import()
        .into_proxy();
    assert_eq!(proxy_b.get(), 0);
    proxy_b.inc();
    assert_eq!(proxy_b.get(), 1);
    proxy_b.inc();
    assert_eq!(proxy_b.get(), 2);

    drop(proxy_a);
    drop(proxy_b);
    drop(port)
}

#[test]
fn service_object_as_argument() {
    init_logger();

    let port = Arc::new(TestPort::new());
    let proxy_a = create_proxy_a(port.clone());

    let service_object_b = Box::new(SimpleB::new()) as Box<dyn B>;
    proxy_a.service_object_as_argument(ServiceRef::Export(ServiceToExport::new(service_object_b)));

    drop(proxy_a);
    drop(port)
}

#[test]
fn recursive_service_object() {
    init_logger();

    let port = Arc::new(TestPort::new());
    let mut proxy_a = create_proxy_a(port.clone());
    let mut proxy_as = Vec::new();
    proxy_as.push(Arc::clone(&proxy_a));

    for i in 0..10 {
        assert_eq!(proxy_a.get_recursion_count(), i);
        proxy_a = proxy_a
            .recursive_service_object()
            .unwrap_import()
            .into_proxy();
        proxy_as.push(Arc::clone(&proxy_a));
    }
    assert_eq!(proxy_a.get_recursion_count(), 10);

    let proxy_b: Box<dyn B> = proxy_a
        .service_object_as_return()
        .unwrap_import()
        .into_proxy();
    proxy_b.inc();
    assert_eq!(proxy_b.get(), 1);

    // proxy_a + proxy_b + recursive 10 proxy_a = 12
    assert_eq!(port.register_len(), 12);

    drop(proxy_as);
    drop(proxy_a);
    drop(proxy_b);
    drop(port)
}
