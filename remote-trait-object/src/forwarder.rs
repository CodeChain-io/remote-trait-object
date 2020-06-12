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

use crate::port::Handler;
use crate::service::Dispatch;
use std::collections::HashMap;

type ServiceId = String;

pub struct ServiceForwarder {
    service_handlers: HashMap<ServiceId, Box<dyn Dispatch>>,
}

impl ServiceForwarder {
    pub fn new() -> Self {
        Self {
            service_handlers: Default::default(),
        }
    }

    pub fn add_service(&mut self, name: ServiceId, service: Box<dyn Dispatch>) {
        let insert_result = self.service_handlers.insert(name.clone(), service);
        if insert_result.is_some() {
            panic!("Duplicated service id {}", name);
        }
    }

    pub fn forward_and_call(&self, input: String) -> String {
        let ParseResult {
            service_name,
            data,
        } = parse(input);
        self.service_handlers[&service_name].dispatch_and_call(data)
    }
}

impl Default for ServiceForwarder {
    fn default() -> Self {
        Self::new()
    }
}

impl Handler for ServiceForwarder {
    fn handle(&self, input: String) -> String {
        self.forward_and_call(input)
    }
}

struct ParseResult {
    service_name: String,
    data: String,
}

fn parse(message: String) -> ParseResult {
    let separator = message
        .find(':')
        .unwrap_or_else(|| panic!("Can't find : to parse service name. Original message: {}", message));

    let service_name = message[..separator].to_string();
    let data = message[separator + 1..].to_string();
    ParseResult {
        service_name,
        data,
    }
}
