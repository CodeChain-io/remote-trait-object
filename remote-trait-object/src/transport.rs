//! Abstractions of a **transport** that carries out an actual communication for `remote-trait-object`.
//!
//! You have to implement these traits in your own requirement, to use `remote-trait-object` over them.
//! It can be ordinary in-process communication, inter-process communication, or even networking over
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
/// All outgoing packets will be delivered to a single instance of this trait, which has been given
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
/// [`TransportSend`] and [`TransportRecv`] must be able to provide such a switch that triggers [`Termination`] error for its own [`send()`] or [`recv()`].
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
