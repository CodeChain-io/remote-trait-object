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

use super::mod_main::main_like as main_main;
use super::mod_ping::main_like as ping_main;
use crate::connection::{create_connection, ConnectionEnd};

fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

/// There are thee entities: commander, main module and ping module.
/// Commander sends "start" message to the main module.
/// If the main module receives "start" message, it sends "ping" to the ping module.
/// If the ping module receives "ping" message, respond "pong".
/// If the main module received "pong" response, send "pong received" to the commander.
#[test]
fn ping() {
    init_logger();

    debug!("ping test start");
    let (main_to_cmd, cmd_to_main) = create_connection();
    let (main_to_ping, ping_to_main) = create_connection();

    debug!("Call main_main");
    let _main_module = main_main(Vec::new(), main_to_cmd, main_to_ping);
    debug!("Call ping_main");
    let _ping_module = ping_main(Vec::new(), ping_to_main);

    let ConnectionEnd {
        sender: to_main,
        receiver: from_main,
    } = cmd_to_main;

    debug!("Send start cmd");
    to_main.send("request:start".to_string()).unwrap();
    debug!("Recv pong response");
    let response = from_main.recv().unwrap();
    assert_eq!(response, "response:pong received".to_string());
    debug!("Test finished");
}
