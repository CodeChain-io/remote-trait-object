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

//! Abstractions of a **transport** that carries out an actual communication for `remote-trait-object`.
//!
//! You have to implement these traits in your own requirement, to use `remote-trait-object` over them.
//! It can be an ordinary in-process communication, an inter-process communication, or even a networking over
//! different machines.

pub(crate) mod multiplex;

/// An error that can be returned in [`send()`] or [`recv()`].
///
/// Note that only `Timeout` and `Termination` will be handled specially by the `remote-trait-object` context.
/// All other errors must be wrapped as `Custom`, and it will be just conveyed to the user.
///
/// [`send()`]: trait.TransportSend.html#tymethod.send
/// [`recv()`]: trait.TransportRecv.html#tymethod.recv
#[derive(Clone, Debug, PartialEq)]
pub enum TransportError {
    /// An error that indicates that your call to `send()` or `recv()` can't be finished within the timeout you set.
    TimeOut,

    /// An error that indicates that you have called [`terminate()`] of the spawned [`Terminate`] from the object you're calling a method of.
    ///
    /// [`Terminate`]: trait.Terminate.html
    /// [`terminate()`]: trait.Terminate.html#tymethod.terminate
    Termination,

    // TODO: Provide an appropriate type for this
    /// An opaque error that will be just passed to the user.
    Custom,
}

/// An abstraction of a sending half of the transport
///
/// All outgoing packet will be delivered to a single instance of this trait, which has been given
/// when [`Context`] is created.
///
/// [`Context`]: ../struct.Context.html
pub trait TransportSend: Sync + Send + std::fmt::Debug {
    /// Sends a packet with an optional timeout.
    fn send(&self, data: &[u8], timeout: Option<std::time::Duration>) -> Result<(), TransportError>;

    /// Creates a terminate switch that can be sent to another thread
    fn create_terminator(&self) -> Box<dyn Terminate>;
}

/// An abstraction of a receiving half of the transport
///
/// All incoming packets will be delivered by a single instance of this trait, which has been given
/// when [`Context`] is created.
///
/// [`Context`]: ../struct.Context.html
pub trait TransportRecv: Send {
    /// Receives a packet with an optional timeout.
    ///
    /// Note that it is not guaranteed to receive remaining data after the counter end has
    /// been closed earlier. You should assume that you will receive Err(Custom) in such case.
    fn recv(&self, timeout: Option<std::time::Duration>) -> Result<Vec<u8>, TransportError>;

    /// Creates a terminate switch that can be sent to another thread
    fn create_terminator(&self) -> Box<dyn Terminate>;
}

/// A switch that can be separately managed by another thread.
///
/// This is the only way to wake up a blocking [`send()`] or [`recv()`] by yourself. (Not by the other end)
/// [`TransportSend`] and [`TransportRecv`] must be able to provide such switch that triggers [`Termination`] error for its own [`send()`] or [`recv()`].
///
/// [`TransportSend`]: trait.TransportSend.html
/// [`TransportRecv`]: trait.TransportRecv.html
/// [`send()`]: trait.TransportSend.html#tymethod.send
/// [`recv()`]: trait.TransportRecv.html#tymethod.recv
/// [`Termination`]: enum.TransportError.html#variant.Termination
pub trait Terminate: Send {
    /// Wakes up block on recv() or send()
    fn terminate(&self);
}
