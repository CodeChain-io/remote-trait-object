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

use crossbeam::channel::{Receiver, Sender};
use std::thread;

pub fn main_like(_args: Vec<String>, receiver: Receiver<String>, main_sender: Sender<String>) {
    start_server(receiver, main_sender);
}

fn start_server(receiver: Receiver<String>, main_sender: Sender<String>) {
    thread::Builder::new()
        .name("ping module".into())
        .spawn(move || {
            let msg = receiver.recv().unwrap();

            if msg == "ping" {
                main_sender.send("pong".to_string()).unwrap();
            }
        })
        .unwrap();
}
