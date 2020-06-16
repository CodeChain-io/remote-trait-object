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

mod context;
mod impls;
mod traits;

use crate::connection::ConnectionEnd;
use cbasesandbox::ipc::Ipc;
use context::Context;
use impls::MainHandler;
use remote_trait_object::{Context as RtoContext, Dispatch, MethodId, Service};
use std::sync::Arc;
use traits::MainInterface;

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

struct StarterService {
    _ctx: Arc<Context>,
    handler: MainHandler,
}

impl StarterService {
    pub fn new(ctx: Arc<Context>) -> Self {
        let handler = MainHandler::new(Arc::clone(&ctx));
        Self {
            _ctx: ctx,
            handler,
        }
    }
}

impl Service for StarterService {}

impl Dispatch for StarterService {
    fn dispatch_and_call(&self, method: MethodId, args: &[u8]) -> Vec<u8> {
        trace!("StarterService received {}({:?}) request", method, args);
        if method == 1 {
            self.handler.start().as_bytes().to_vec()
        } else {
            panic!("unexpected msg in main module from cmd {}({:?})", method, args)
        }
    }
}

fn start_server<IPC: Ipc>(with_cmd: ConnectionEnd<IPC>, with_ping: ConnectionEnd<IPC>) -> Arc<Context> {
    let ctx = Arc::new(Context::new());
    let cmd_rto = {
        let ConnectionEnd {
            receiver: from_cmd,
            sender: to_cmd,
        } = with_cmd;
        let cmd_rto = RtoContext::new(to_cmd, from_cmd);
        cmd_rto
            .get_port()
            .upgrade()
            .unwrap()
            .register("Singleton".to_owned(), Box::new(StarterService::new(Arc::clone(&ctx))));
        cmd_rto
    };

    let ping_rto = {
        let ConnectionEnd {
            receiver: from_ping,
            sender: to_ping,
        } = with_ping;
        RtoContext::new(to_ping, from_ping)
    };

    ctx.initialize_rtos(cmd_rto, ping_rto);
    ctx
}
