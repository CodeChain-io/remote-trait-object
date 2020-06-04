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

use crossbeam::channel::{self, Receiver};
use std::thread;

pub struct MultiplexResult {
    pub request_recv: Receiver<String>,
    pub response_recv: Receiver<String>,
    pub multiplexer: Multiplexer,
}

pub struct Multiplexer {
    _receiver_thread: Option<thread::JoinHandle<()>>,
}

impl Multiplexer {
    pub fn multiplex(ipc_recv: Receiver<String>) -> MultiplexResult {
        let (request_send, request_recv) = channel::bounded(1);
        let (response_send, response_recv) = channel::bounded(1);

        let receiver_thread = thread::Builder::new()
            .name("multiplexer".into())
            .spawn(move || loop {
                let original_message = ipc_recv.recv().unwrap();
                // FIXME: parsing is not the role of the Multiplexer.
                let message = parse(original_message.clone());
                match message {
                    Some(ParseResult::Request(request)) => request_send.send(request).unwrap(),
                    Some(ParseResult::Response(response)) => response_send.send(response).unwrap(),
                    None => {
                        panic!("Receved invalid message {}", original_message);
                    }
                }
            })
            .unwrap();

        MultiplexResult {
            request_recv,
            response_recv,
            multiplexer: Multiplexer {
                _receiver_thread: Some(receiver_thread),
            },
        }
    }
}

enum ParseResult {
    Request(String),
    Response(String),
}

fn parse(message: String) -> Option<ParseResult> {
    let request_prefix = "request:";
    let response_prefix = "response:";
    if message.starts_with(request_prefix) {
        Some(ParseResult::Request(
            message.trim_start_matches(request_prefix).to_string(),
        ))
    } else if message.starts_with(response_prefix) {
        Some(ParseResult::Response(
            message.trim_start_matches(response_prefix).to_string(),
        ))
    } else {
        None
    }
}
