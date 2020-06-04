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

#[cfg(test)]
use crossbeam::channel;
use crossbeam::channel::{Receiver, Sender};

#[cfg(test)]
pub fn create_connection() -> (ConnectionEnd, ConnectionEnd) {
    let (send_to_a, recv_in_a) = channel::bounded(1);
    let (send_to_b, recv_in_b) = channel::bounded(1);

    (
        ConnectionEnd {
            sender: send_to_a,
            receiver: recv_in_b,
        },
        ConnectionEnd {
            sender: send_to_b,
            receiver: recv_in_a,
        },
    )
}

pub struct ConnectionEnd {
    pub sender: Sender<String>,
    pub receiver: Receiver<String>,
}
