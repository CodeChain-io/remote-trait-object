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
use crossbeam::channel;

/// There are thee entities: commander, main module and ping module.
/// Commander sends "start" message to the main module.
/// If the main module receives "start" message, it sends "ping" to the ping module.
/// If the ping module receives "ping" message, respond "pong".
/// If the main module received "pong" response, send "pong received" to the commander.
#[test]
fn ping() {
    let (cmd_sender, cmd_receiver) = channel::bounded(1);
    let (main_sender, main_receiver) = channel::bounded(1);
    let (ping_sender, ping_receiver) = channel::bounded(1);
    main_main(Vec::new(), main_receiver, cmd_sender, ping_sender);
    ping_main(Vec::new(), ping_receiver, main_sender.clone());

    main_sender.send("start".to_string()).unwrap();
    let response = cmd_receiver.recv().unwrap();
    assert_eq!(response, "pong received".to_string());
}
