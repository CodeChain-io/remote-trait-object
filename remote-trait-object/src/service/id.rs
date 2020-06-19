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

use super::MethodId;
use linkme::distributed_slice;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const ID_ORDERING: std::sync::atomic::Ordering = std::sync::atomic::Ordering::SeqCst;
pub type MethodIdAtomic = std::sync::atomic::AtomicU32;

// linkme crate smartly collects all the registrations generated by the proc-macro
// into a sinlge array in the link time.
// Note that too long linkme-related variable name would cause serious compiler error in MacOS
// So we deliberately make it have a short name

// Id of methods in services.
// Note that here the two strings mean (trait name, method name)
// Also you can skip calling this, then the method id will be set up for default value
// decided by the order of declaration.
type MethodIdentifierSetter = fn(id: MethodId);
#[distributed_slice]
pub static MID_REG: [(&'static str, &'static str, MethodIdentifierSetter)] = [..];

/// This will be provided by the user who cares the compatability between already-compiled service traits.
#[derive(PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct IdMap {
    // This is system-wide; All module will get same ones
    pub method_map: Option<HashMap<(String, String), MethodId>>,
}

/// This is supposed to be called only once during the entire lifetime of the process.
/// However it is ok to call multiple times if the IdMap is identical, especially in the
/// tests where each test share that static id list
pub fn setup_identifiers(descriptor: &IdMap) {
    // distributed_slices integrity test
    {
        let mut bucket: HashSet<(String, String)> = HashSet::new();
        for (ident1, ident2, _) in MID_REG {
            bucket.insert(((*ident1).to_owned(), (*ident2).to_owned()));
        }
        assert_eq!(
            bucket.len(),
            MID_REG.len(),
            "The service traits that this binary involved are not named;
        You have provided multiple traits with an identical name"
        );
    }

    // method ids have default values decided by the order, so it is ok to leave them in an ordinary case.
    if let Some(map) = descriptor.method_map.as_ref() {
        for (trait_name, method_name, setter) in MID_REG {
            setter(
                *map.get(&((*trait_name).to_owned(), (*method_name).to_owned())).expect("Invalid handle descriptor"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(non_upper_case_globals)]
    static ID_METHOD_MyTrait_mymethod: crate::macro_env::MethodIdAtomic = crate::macro_env::MethodIdAtomic::new(1);
    #[linkme::distributed_slice(crate::macro_env::MID_REG)]
    #[allow(non_upper_case_globals)]
    static ID_METHOD_ENTRY_MyTrait_mymethod: (&'static str, &'static str, fn(id: crate::macro_env::MethodId)) =
        ("MyTrait", "mymethod", id_method_setter_MyTrait_mymethod);
    #[allow(non_snake_case)]
    fn id_method_setter_MyTrait_mymethod(id: crate::macro_env::MethodId) {
        ID_METHOD_MyTrait_mymethod.store(id, crate::macro_env::ID_ORDERING);
    }
    #[test]
    fn setup() {
        let id_map: HashMap<(String, String), MethodId> =
            [(("MyTrait".to_owned(), "mymethod".to_owned()), 123)].iter().cloned().collect();
        let id_map = IdMap {
            method_map: Some(id_map),
        };
        setup_identifiers(&id_map);
        assert_eq!(ID_METHOD_MyTrait_mymethod.load(crate::macro_env::ID_ORDERING), 123);
    }
}
