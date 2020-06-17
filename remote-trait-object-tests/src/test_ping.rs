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

mod mod_main;
mod mod_ping;

use self::mod_main::main_like as main_main;
use self::mod_ping::main_like as ping_main;
use crate::connection::{create_connection, ConnectionEnd};
use cbasesandbox::ipc::intra::Intra;
use cbasesandbox::ipc::unix_socket::DomainSocket;
use cbasesandbox::ipc::{Ipc, IpcRecv, IpcSend};
use remote_trait_object::{Packet, PacketView};
use std::time::Duration;

fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn ping_intra() {
    test_ping::<Intra>();
}

#[test]
fn ping_unix_socket() {
  test_ping::<DomainSocket>();
}

/// There are thee entities: commander, main module and ping module.
/// Commander sends "start" message to the main module.
/// If the main module receives "start" message, it sends "ping" to the ping module.
/// If the ping module receives "ping" message, respond "pong".
/// If the main module received "pong" response, send "pong received" to the commander.
fn test_ping<IPC: Ipc + 'static>() {
    init_logger();

    debug!("ping test start");
    let (main_to_cmd, cmd_to_main) = create_connection::<IPC>();
    let (main_to_ping, ping_to_main) = create_connection::<IPC>();

    debug!("Call main_main");
    let _main_module = main_main(Vec::new(), main_to_cmd, main_to_ping);
    debug!("Call ping_main");
    let _ping_module = ping_main(Vec::new(), ping_to_main);

    let ConnectionEnd {
        sender: to_main,
        receiver: from_main,
    } = cmd_to_main;

    debug!("Send start cmd");
    // FIXME: 0 is temporary value assuming singleton service object
    let packet = Packet::new_request(0, 1, &[]);
    to_main.send(&packet.into_vec());
    debug!("Recv pong response");
    let response = from_main.recv(Some(Duration::from_secs(1))).unwrap();
    let response_packet = PacketView::new(&response);
    assert_eq!(response_packet.data(), b"pong received");
    debug!("Test finished");
}
