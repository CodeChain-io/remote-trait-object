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

use crossbeam::channel::{
    bounded, Receiver,
    RecvTimeoutError::{Disconnected, Timeout},
    Sender,
};
use parking_lot::Mutex;

/// Blocking concurrent Queue. (Crossbeam's queue doens't block)
/// Please use the queue with Arc
#[derive(Debug)]
pub struct Queue<T> {
    // sender is None only when the close is called
    sender: Mutex<Option<Sender<T>>>,
    // receiver is None only when the close is called
    recver: Mutex<Option<Receiver<T>>>,
}

impl<T> Queue<T> {
    pub fn new(size: usize) -> Self {
        let (sender, recver) = bounded(size);
        Queue {
            sender: Mutex::new(Some(sender)),
            recver: Mutex::new(Some(recver)),
        }
    }

    pub fn push(&self, x: T) -> Result<(), QueueClosed> {
        let guard = self.sender.lock();
        let sender = guard.as_ref().ok_or(QueueClosed)?;
        sender.send(x).map_err(|_| QueueClosed)
    }
    pub fn pop(&self, timeout: Option<std::time::Duration>) -> Result<T, PopError> {
        let guard = self.recver.lock();
        let recver = guard.as_ref().ok_or(PopError::QueueClosed)?;
        if let Some(duration) = timeout {
            recver.recv_timeout(duration).map_err(|err| match err {
                Timeout => PopError::Timeout,
                Disconnected => PopError::QueueClosed,
            })
        } else {
            recver.recv().map_err(|_| PopError::QueueClosed)
        }
    }
}

#[derive(Debug)]
pub struct QueueClosed;

#[derive(Debug)]
pub enum PopError {
    Timeout,
    QueueClosed,
}
