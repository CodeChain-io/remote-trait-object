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
