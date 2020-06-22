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
use impls::SimpleMain;
use remote_trait_object::Context as RtoContext;
use std::sync::Arc;
use traits::MainDispatcher;

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

fn start_server<IPC: Ipc>(with_cmd: ConnectionEnd<IPC>, with_ping: ConnectionEnd<IPC>) -> Arc<Context> {
    let ctx = Arc::new(Context::new());
    let cmd_rto = {
        let ConnectionEnd {
            receiver: from_cmd,
            sender: to_cmd,
        } = with_cmd;
        let cmd_rto = RtoContext::new(to_cmd, from_cmd);
        // TODO: use this
        let _handle_to_export = cmd_rto
            .get_port()
            .upgrade()
            .unwrap()
            // TODO: you shouldn't manually register dispatcher. Use export macro.
            .register(Arc::new(MainDispatcher::new(Arc::new(SimpleMain::new(Arc::clone(&ctx))))));
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
