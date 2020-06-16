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

use super::context::Context;
use super::traits::MainInterface;
use crate::mod_ping::requester::{PingInterface, PingRequester};
use remote_trait_object::{Dispatch, MethodId, Service};
use std::sync::Arc;

pub struct MainHandler {
    ctx: Arc<Context>,
}

impl MainHandler {
    pub fn new(ctx: Arc<Context>) -> Self {
        Self {
            ctx,
        }
    }
}

impl MainInterface for MainHandler {
    fn start(&self) -> String {
        let ping_requester = PingRequester::new(self.ctx.ping_rto());
        let pong = ping_requester.ping();
        if pong == "pong" {
            "pong received".to_string()
        } else {
            format!("unexpected {} received", pong)
        }
    }
}

impl Dispatch for MainHandler {
    fn dispatch_and_call(&self, method: MethodId, args: &[u8]) -> Vec<u8> {
        remote_trait_object::service_dispatch!(MainInterface, self, method, args)
    }
}

impl Service for MainHandler {}
