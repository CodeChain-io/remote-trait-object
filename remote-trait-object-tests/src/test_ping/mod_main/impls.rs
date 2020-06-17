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

use super::super::mod_ping::{Ping, PingRemote};
use super::context::Context;
use super::traits::Main;
use remote_trait_object::{Handle, Service};
use std::sync::Arc;

pub struct SimpleMain {
    ctx: Arc<Context>,
}

impl SimpleMain {
    pub fn new(ctx: Arc<Context>) -> Self {
        Self {
            ctx,
        }
    }
}

impl Main for SimpleMain {
    fn start(&self) -> String {
        // FIXME: Remote struct can be made only by import
        let ping_requester = PingRemote {
            handle: Handle {
                port: self.ctx.ping_rto().get_port(),
                id: 0,
            },
        };
        let pong = ping_requester.ping();
        if pong == "pong" {
            "pong received".to_string()
        } else {
            format!("unexpected {} received", pong)
        }
    }
}

impl Service for SimpleMain {}
