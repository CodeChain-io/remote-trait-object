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

pub mod intra;

use once_cell::sync::OnceCell;
use parking_lot::Mutex;
pub use remote_trait_object::ipc::*;

pub trait Ipc: IpcSend + IpcRecv {
    /// Generate two configurations
    /// which will be feeded to Ipc::new(),
    /// for both two ends in two different processes, to initialize each IPC end.
    /// Note that both sides' new() must be called concurrently; they will be completed only if
    /// both run at the same time.
    fn arguments_for_both_ends() -> (Vec<u8>, Vec<u8>);

    type SendHalf: IpcSend;
    type RecvHalf: IpcRecv;

    /// Constructs itself with an opaque data that would have been transported by some IPC
    fn new(data: Vec<u8>) -> Self;
    /// split itself into Send-only and Recv-only. This is helpful for a threading
    /// When you design both halves, you might consider who's in charge of cleaning up things.
    /// Common implementation is making both to have Arc<SomethingDroppable>.
    fn split(self) -> (Self::SendHalf, Self::RecvHalf);
}
