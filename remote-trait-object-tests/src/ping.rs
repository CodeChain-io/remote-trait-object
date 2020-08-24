use remote_trait_object::*;
use std::sync::{Arc, Barrier};
use std::thread;

#[service]
pub trait Hello: Service {
    fn hey(&self) -> ServiceRef<dyn Ping>;
}

struct SimpleHello {
    barrier: Arc<Barrier>,
}

impl Service for SimpleHello {}

impl Hello for SimpleHello {
    fn hey(&self) -> ServiceRef<dyn Ping> {
        ServiceRef::create_export(Box::new(SimplePing {
            barrier: Arc::clone(&self.barrier),
        }) as Box<dyn Ping>)
    }
}

#[service]
pub trait Ping: Service {
    fn ping(&self);
    fn ping_mut(&mut self);
    fn ping_barrier(&self);
}

struct SimplePing {
    barrier: Arc<Barrier>,
}

impl Service for SimplePing {}

impl Ping for SimplePing {
    fn ping(&self) {}

    fn ping_mut(&mut self) {}

    fn ping_barrier(&self) {
        self.barrier.wait();
    }
}

#[allow(clippy::type_complexity)]
fn run(barrier: Arc<Barrier>) -> ((Context, ServiceToImport<dyn Hello>), (Context, ServiceToImport<dyn Hello>)) {
    let crate::transport::TransportEnds {
        recv1,
        send1,
        recv2,
        send2,
    } = crate::transport::create();
    (
        Context::with_initial_service(
            Config::default_setup(),
            send1,
            recv1,
            ServiceToExport::new(Box::new(SimpleHello {
                barrier: Arc::clone(&barrier),
            }) as Box<dyn Hello>),
        ),
        Context::with_initial_service(
            Config::default_setup(),
            send2,
            recv2,
            ServiceToExport::new(Box::new(SimpleHello {
                barrier,
            }) as Box<dyn Hello>),
        ),
    )
}

#[test]
fn ping1() {
    let barrier = Arc::new(Barrier::new(1));
    let ((_ctx1, hello1), (_ctx2, hello2)) = run(Arc::clone(&barrier));
    let hello1: Box<dyn Hello> = hello1.into_proxy();
    let hello2: Box<dyn Hello> = hello2.into_proxy();

    let ping1: Box<dyn Ping> = hello1.hey().unwrap_import().into_proxy();
    let ping2: Box<dyn Ping> = hello2.hey().unwrap_import().into_proxy();

    ping1.ping();
    ping2.ping();

    drop(hello1);
    drop(hello2);
}

#[test]
fn ping_concurrent1() {
    let n = 6;
    for _ in 0..100 {
        let barrier = Arc::new(Barrier::new(n + 1));
        let ((_ctx1, hello1), (_ctx2, hello2)) = run(Arc::clone(&barrier));
        let hello1: Box<dyn Hello> = hello1.into_proxy();
        let hello2: Box<dyn Hello> = hello2.into_proxy();

        let pings: Vec<Box<dyn Ping>> = (0..n).map(|_| hello2.hey().unwrap_import().into_proxy()).collect();
        let joins: Vec<thread::JoinHandle<()>> = pings
            .into_iter()
            .map(|ping| {
                thread::spawn(move || {
                    ping.ping_barrier();
                })
            })
            .collect();
        barrier.wait();
        for join in joins {
            join.join().unwrap();
        }

        drop(hello1);
        drop(hello2);
    }
}

#[test]
fn ping_concurrent2() {
    let n = 6;
    for _ in 0..100 {
        let barrier = Arc::new(Barrier::new(n + 1));
        let ((_ctx1, hello1), (_ctx2, hello2)) = run(Arc::clone(&barrier));
        let hello1: Box<dyn Hello> = hello1.into_proxy();
        let hello2: Box<dyn Hello> = hello2.into_proxy();

        let ping: Arc<dyn Ping> = hello2.hey().unwrap_import().into_proxy();

        let joins: Vec<thread::JoinHandle<()>> = (0..n)
            .map(|_| {
                let ping_ = Arc::clone(&ping);
                thread::spawn(move || {
                    ping_.ping_barrier();
                })
            })
            .collect();
        barrier.wait();
        for join in joins {
            join.join().unwrap();
        }

        drop(hello1);
        drop(hello2);
    }
}

#[test]
#[should_panic(expected = "You invoked a method of a null proxy object.")]
fn null_proxy() {
    let barrier = Arc::new(Barrier::new(1));
    let ((ctx1, _), (_ctx2, _)) = run(Arc::clone(&barrier));
    let null_handle = remote_trait_object::raw_exchange::HandleToExchange::create_null();
    let null_proxy: Box<dyn Ping> = remote_trait_object::raw_exchange::import_service_from_handle(&ctx1, null_handle);
    null_proxy.ping();
}

#[test]
#[should_panic(expected = "You invoked a method of a null proxy object.")]
fn null_proxy2() {
    let null_proxy: Box<dyn Ping> = remote_trait_object::raw_exchange::import_null_proxy();
    null_proxy.ping();
}
