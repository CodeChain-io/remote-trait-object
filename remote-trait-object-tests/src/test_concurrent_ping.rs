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

use crate::transport::{IntraRecv, IntraSend, TransportEnds};
use remote_trait_object::{Config, Context, Packet};
use std::sync::mpsc;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

// This test tests concurrency of Port.
// Makes a barrier that needs 5 wait calls.
// The test succeeds only when the below conditions are met.
// 1. Port's client sends 4 packets in parallel.
// 2. Port's server calls 4 handlers in parallel.
#[test]
fn ping() {
    init_logger();

    panic_after(std::time::Duration::from_secs(1), || {
        debug!("ping test start");
        let TransportEnds {
            send1,
            recv1,
            send2,
            recv2,
        } = crate::transport::create();

        let number_of_calls = 4;
        let wait_before_test_end = 1;

        // We use barrier to check concurrency
        // This test blocks if the packets are not handled concurrently.
        let barrier = Arc::new(Barrier::new(number_of_calls + wait_before_test_end));

        let _ping_module = create_ping_module(send2, recv2, Arc::clone(&barrier));

        let cmd_to_ping_rto = Context::new(Config::default_setup(), send1, recv1);
        let mut handles = Vec::new();

        for i in 0..number_of_calls {
            let port = cmd_to_ping_rto.get_port().upgrade().unwrap();

            let joiner = thread::Builder::new()
                .name(format!("ping sender {}", i))
                .spawn(move || {
                    // FIXME: 0 is temporary value assuming singleton service object
                    let request = Packet::new_request(0, 1, &[]);
                    let response = port.call(request.view());
                    assert_eq!(response.data(), b"pong");
                })
                .unwrap();
            handles.push(joiner);
        }

        barrier.wait();

        for handle in handles {
            handle.join().unwrap();
        }
    });
}

fn create_ping_module(transport_send: IntraSend, transport_recv: IntraRecv, barrier: Arc<Barrier>) -> Context {
    let cmd_rto = Context::new(Config::default_setup(), transport_send, transport_recv);
    let port = cmd_rto.get_port().upgrade().unwrap();
    let _handle_to_export = port.register(Arc::new(move |_method: u32, _args: &[u8]| {
        // Wait until barrier.wait is called in concurrently
        barrier.wait();
        b"pong".to_vec()
    }));

    cmd_rto
}

/// Copied from https://github.com/rust-lang/rfcs/issues/2798#issuecomment-552949300
fn panic_after<T, F>(d: Duration, f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T,
    F: Send + 'static, {
    let (done_tx, done_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let val = f();
        done_tx.send(()).expect("Unable to send completion signal");
        val
    });

    match done_rx.recv_timeout(d) {
        Ok(_) => handle.join().expect("Thread panicked"),
        Err(err) => panic!("Thread took too long {}", err),
    }
}
