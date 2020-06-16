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
use remote_trait_object::{Context, Packet};

pub struct PingRequester<'a> {
    ping_rto: &'a Context,
}

impl<'a> PingRequester<'a> {
    pub fn new(ping_rto: &'a Context) -> Self {
        Self {
            ping_rto,
        }
    }
}

impl<'a> PingInterface for PingRequester<'a> {
    fn ping(&self) -> String {
        let service_name = "Singleton";
        let packet = Packet::new_request(service_name.to_string(), 1, &[]);
        let response = self.ping_rto.get_port().upgrade().unwrap().call(packet.view());
        String::from_utf8(response.data().to_vec()).unwrap()
    }
}
