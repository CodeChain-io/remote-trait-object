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

pub use super::traits::PingInterface;
use remote_trait_object::Context;

pub struct PingRequester<'a> {
    port: &'a Context,
}

impl<'a> PingRequester<'a> {
    pub fn new(port: &'a Context) -> Self {
        Self {
            port,
        }
    }
}

impl<'a> PingInterface for PingRequester<'a> {
    fn ping(&self) -> String {
        let service_name = "Singleton";
        let method_name = "ping";
        self.port.get_port().upgrade().unwrap().call(format!("{}:{}", service_name, method_name))
    }
}
