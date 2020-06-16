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

use crate::packet::PacketView;

pub trait Handler: Send + Sync {
    fn handle(&self, input: PacketView) -> Vec<u8>;
}

impl<F> Handler for F
where
    F: Fn(PacketView) -> Vec<u8> + Send + Sync,
{
    fn handle(&self, input: PacketView) -> Vec<u8> {
        self(input)
    }
}
