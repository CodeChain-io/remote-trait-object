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

mod impls;
mod traits;

use crate::connection::ConnectionEnd;
use cbasesandbox::ipc::Ipc;
use impls::SimplePong;
use remote_trait_object::Context;
use std::sync::Arc;
use traits::PingHandler;
pub use traits::{Ping, PingRemote};

pub fn main_like<IPC>(_args: Vec<String>, with_main: ConnectionEnd<IPC>) -> PingModule
where
    IPC: Ipc, {
    let ConnectionEnd {
        receiver: from_main,
        sender: to_main,
    } = with_main;

    let main_rto = Context::new(to_main, from_main);
    // TODO: use this
    let _handle_to_export =
        main_rto.get_port().upgrade().unwrap().register(Arc::new(PingHandler::new(Arc::new(SimplePong {}))));

    PingModule {
        _main_rto: main_rto,
    }
}

pub struct PingModule {
    _main_rto: Context,
}
