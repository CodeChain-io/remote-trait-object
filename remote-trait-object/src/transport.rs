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

pub(crate) mod multiplex;

#[derive(Debug, PartialEq)]
pub enum SendError {
    Custom,
}

pub trait TransportSend: Send {
    /// It might block until counterparty's recv(). Even if not, the order is still guaranteed.
    fn send(&self, data: &[u8]) -> Result<(), SendError>;
}

#[derive(Debug, PartialEq)]
pub enum RecvError {
    TimeOut,
    Termination,
}

pub trait Terminate: Send {
    /// Wake up block on recv
    fn terminate(&self);
}

pub trait TransportRecv: Send {
    /// Returns Err only for the timeout or termination wake-up(otherwise panic).
    ///
    /// Note that it is not guaranteed to receive remaining data after the counter end has
    /// been closed earlier. You should assume that you will receive Err(Termination) in such case.
    fn recv(&self, timeout: Option<std::time::Duration>) -> Result<Vec<u8>, RecvError>;

    /// Create a terminate switch that can be sent to another thread
    fn create_terminator(&self) -> Box<dyn Terminate>;
}
