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

use crate::connection::ConnectionEnd;
use cbasesandbox::ipc::Ipc;
use parking_lot::Mutex;
use remote_trait_object::{Port, ServiceForwarder, ServiceHandler};
use std::sync::Arc;

pub fn main_like<IPC: Ipc>(
    _args: Vec<String>,
    with_cmd: ConnectionEnd<IPC>,
    with_ping: ConnectionEnd<IPC>,
) -> MainModule {
    let context = start_server(with_cmd, with_ping);
    MainModule {
        _context: context,
    }
}

pub struct MainModule {
    _context: Arc<Context>,
}

struct Context {
    cmd_port: Mutex<Option<Port>>,
    ping_port: Mutex<Option<Port>>,
}

impl Context {
    fn new() -> Self {
        Context {
            cmd_port: Default::default(),
            ping_port: Default::default(),
        }
    }
}

struct StarterService {
    ctx: Arc<Context>,
}

impl StarterService {
    pub fn new(ctx: Arc<Context>) -> Self {
        Self {
            ctx,
        }
    }
}

impl ServiceHandler for StarterService {
    fn call(&self, msg: String) -> String {
        if msg == "start" {
            let ping_port = self.ctx.ping_port.lock();
            let pong = ping_port.as_ref().unwrap().call("ping:ping".to_string());
            if pong == "pong" {
                "pong received".to_string()
            } else {
                format!("unexpected {} received", pong)
            }
        } else {
            panic!("unexpected msg in main module from cmd {}", msg)
        }
    }
}

fn start_server<IPC: Ipc>(with_cmd: ConnectionEnd<IPC>, with_ping: ConnectionEnd<IPC>) -> Arc<Context> {
    let ctx = Arc::new(Context::new());
    let cmd_port = {
        let mut service_forwarder = ServiceForwarder::new();
        service_forwarder.add_service("main".to_string(), Box::new(StarterService::new(Arc::clone(&ctx))));

        let ConnectionEnd {
            receiver: from_cmd,
            sender: to_cmd,
        } = with_cmd;
        Port::new(to_cmd, from_cmd, service_forwarder)
    };

    let ping_port = {
        let ConnectionEnd {
            receiver: from_ping,
            sender: to_ping,
        } = with_ping;
        Port::new(to_ping, from_ping, |msg| {
            panic!("main do not expect receiving packet from ping. msg: {}", msg);
        })
    };

    *ctx.cmd_port.lock() = Some(cmd_port);
    *ctx.ping_port.lock() = Some(ping_port);
    ctx
}
