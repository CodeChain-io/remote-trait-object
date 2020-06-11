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
use remote_trait_object::Port;

pub fn main_like<IPC>(_args: Vec<String>, with_main: ConnectionEnd<IPC>) -> PingModule
where
    IPC: Ipc, {
    let port_to_main = start_server(with_main);
    PingModule {
        _port_to_main: port_to_main,
    }
}

pub struct PingModule {
    _port_to_main: Port,
}

fn start_server<IPC: Ipc>(with_main: ConnectionEnd<IPC>) -> Port {
    let ConnectionEnd {
        receiver: from_main,
        sender: to_main,
    } = with_main;
    Port::new(to_main, from_main, |msg| {
        if msg == "ping" {
            "pong".to_string()
        } else {
            panic!("Unexpected message in ping from main {}", msg)
        }
    })
}
