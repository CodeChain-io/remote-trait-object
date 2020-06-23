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

mod control_loop;

use cbasesandbox::ipc::intra::Intra;
use cbasesandbox::ipc::{Ipc, IpcRecv, IpcSend};
use remote_trait_object::HandleToExchange;
use std::fmt;
use std::sync::Arc;
use std::thread;

fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn test_module() {
    init_logger();

    let (main_control, main_args) = Intra::arguments_for_both_ends();
    let (ping_control, ping_args) = Intra::arguments_for_both_ends();
    let main_join = main::run_main_module(vec!["main".to_string(), hex::encode(main_args)]);
    info!("main module created");
    let ping_join = ping::run_ping_module(vec!["ping".to_string(), hex::encode(ping_args)]);
    info!("ping module created");

    let main_control_ipc = Arc::new(Intra::new(main_control));
    let ping_control_ipc = Arc::new(Intra::new(ping_control));

    {
        info!("Read init for main");
        assert_eq!(main_control_ipc.recv(None).unwrap(), b"#INIT\0");
        info!("Read init for ping");
        assert_eq!(ping_control_ipc.recv(None).unwrap(), b"#INIT\0");

        info!("Send args to main");
        send(&main_control_ipc, &b"args".to_vec());
        info!("Send args to ping");
        send(&ping_control_ipc, &b"args".to_vec());

        let (to_ping, to_main) = Intra::arguments_for_both_ends();
        {
            let main_control_ipc_ = Arc::clone(&main_control_ipc);
            let join1 = thread::spawn(move || {
                info!("send link to main");
                send(&main_control_ipc_, &"link".to_string());
                send(&main_control_ipc_, &("ping", "Intra", to_ping));
                recv_done(&main_control_ipc_);
            });

            let ping_control_ipc_ = Arc::clone(&ping_control_ipc);
            let join2 = thread::spawn(move || {
                info!("send link to ping");
                send(&ping_control_ipc_, &"link".to_string());
                send(&ping_control_ipc_, &("main", "Intra", to_main));
                recv_done(&ping_control_ipc_);
            });

            join1.join().unwrap();
            join2.join().unwrap();
        }

        send(&ping_control_ipc, &"handle_export".to_string());
        send(&ping_control_ipc, &("main", "ping"));
        let handle: HandleToExchange = recv(&ping_control_ipc);
        recv_done(&ping_control_ipc);

        send(&main_control_ipc, &"handle_import".to_string());
        send(&main_control_ipc, &("ping", "ping", handle));
        recv_done(&main_control_ipc);

        send(&main_control_ipc, &"start".to_string());
        // The line below is the most important line in this test.
        assert_recv_msg(&main_control_ipc, &"ping and pong received".to_string());
        recv_done(&main_control_ipc);

        send(&main_control_ipc, &"quit".to_string());
        main_control_ipc.send(b"#TERMINATE\0");
        send(&ping_control_ipc, &"quit".to_string());
        ping_control_ipc.send(b"#TERMINATE\0");

        info!("DONE");
    }

    main_join.join().unwrap();
    ping_join.join().unwrap();
}

fn recv<IPC: Ipc, T: serde::de::DeserializeOwned>(ipc: impl AsRef<IPC>) -> T {
    let bytes = ipc.as_ref().recv(None).unwrap();
    serde_cbor::from_slice(&bytes).unwrap()
}

fn recv_done<IPC: Ipc>(ipc: impl AsRef<IPC>) {
    let done: String = recv(ipc);
    assert_eq!(done, "done");
}

fn assert_recv_msg<IPC: Ipc, T>(ipc: impl AsRef<IPC>, msg: &T)
where
    T: serde::de::DeserializeOwned + PartialEq + fmt::Debug, {
    let received: T = recv(ipc);
    assert_eq!(&received, msg);
}

fn send<IPC: Ipc + 'static, T: serde::Serialize>(ipc: impl AsRef<IPC>, data: &T) {
    ipc.as_ref().send(&serde_cbor::to_vec(data).unwrap());
}

mod main {
    use super::control_loop;
    use super::ping::Ping;
    use cbasesandbox::ipc::intra::Intra;
    use once_cell::sync::OnceCell;
    use remote_trait_object::{import_service, Context, HandleToExchange};
    use std::sync::Arc;
    use std::thread;

    struct MainModule {
        context: Arc<MainContext>,
    }

    pub fn run_main_module(args: Vec<String>) -> thread::JoinHandle<()> {
        thread::Builder::new()
            .name("test_module::main_module".to_string())
            .spawn(|| {
                control_loop::module_control_loop::<Intra, MainModule>(args);
            })
            .unwrap()
    }

    impl control_loop::Bootstrap for MainModule {
        fn new(_args: Vec<u8>) -> Self {
            Self {
                context: Arc::new(MainContext {
                    ping_rto: Default::default(),
                }),
            }
        }

        fn export(&mut self, _context: &Context, _service_name: String) -> HandleToExchange {
            unreachable!();
        }

        fn import(&mut self, context: &Context, service_name: String, exchange: HandleToExchange) {
            if service_name == "ping" {
                self.context
                    .ping_rto
                    .set(import_service!(Ping, context, exchange))
                    .expect("Imported more than one ping service");
            } else {
                unreachable!();
            }
        }

        fn start(&self) -> String {
            let ping = self.context.ping_rto.get().unwrap().ping();
            let pong = self.context.ping_rto.get().unwrap().pong();
            assert_eq!(ping, "ping");
            assert_eq!(pong, "pong");
            "ping and pong received".to_string()
        }
    }

    struct MainContext {
        ping_rto: OnceCell<Arc<dyn Ping>>,
    }
}

mod ping {
    use super::control_loop;
    use cbasesandbox::ipc::intra::Intra;
    use remote_trait_object::{export_service, Context, HandleToExchange, Service};
    use std::fmt;
    use std::sync::Arc;
    use std::thread;

    struct PingModule {}

    pub fn run_ping_module(args: Vec<String>) -> thread::JoinHandle<()> {
        thread::Builder::new()
            .name("test_module::ping_module".to_string())
            .spawn(|| control_loop::module_control_loop::<Intra, PingModule>(args))
            .unwrap()
    }

    impl control_loop::Bootstrap for PingModule {
        fn new(_args: Vec<u8>) -> Self {
            Self {}
        }

        fn export(&mut self, context: &Context, service_name: String) -> HandleToExchange {
            if service_name == "ping" {
                export_service!(Ping, context, Arc::new(PingImpl {}))
            } else {
                unreachable!();
            }
        }

        fn import(&mut self, _context: &Context, _service_name: String, _exchange: HandleToExchange) {
            unreachable!();
        }
    }

    #[rto_macro::service]
    pub trait Ping: Service + fmt::Debug {
        fn ping(&self) -> String;
        fn pong(&self) -> String;
    }

    #[derive(Debug)]
    struct PingImpl {}

    impl Ping for PingImpl {
        fn ping(&self) -> String {
            "ping".to_string()
        }
        fn pong(&self) -> String {
            "pong".to_string()
        }
    }

    impl Service for PingImpl {}
}
