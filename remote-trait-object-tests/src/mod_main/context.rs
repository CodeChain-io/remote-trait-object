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

use once_cell::sync::OnceCell;
use remote_trait_object::Context as RtoContext;
use std::fmt;
use std::fmt::Debug;

pub struct Context {
    inner: OnceCell<ContextInner>,
}

struct ContextInner {
    pub cmd_rto: RtoContext,
    pub ping_rto: RtoContext,
}

impl Debug for ContextInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Context")
    }
}

impl Context {
    pub fn new() -> Self {
        Context {
            inner: Default::default(),
        }
    }

    pub fn initialize_rtos(&self, cmd_rto: RtoContext, ping_rto: RtoContext) {
        self.inner
            .set(ContextInner {
                cmd_rto,
                ping_rto,
            })
            .expect("initialize_rtos should be called only once");
    }

    pub fn ping_rto(&self) -> &RtoContext {
        &self.inner.get().expect("Context is initalized").ping_rto
    }
}
