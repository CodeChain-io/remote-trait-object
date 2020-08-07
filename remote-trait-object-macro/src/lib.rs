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

//! This crate provides one core attribute procedural macro that is attached to the trait that you want to use as a service.
//! See more details in `remote-trait-object` crate.

#[macro_use]
extern crate quote;

mod helper;
mod service;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

/// Those necessary components for the macro is specially exported in the remote-trait-object.
/// The macro will always specify full path using this.
fn create_env_path() -> syn::Path {
    syn::parse2(quote! {remote_trait_object::macro_env}).unwrap()
}

/// It generates all necessary helper `struct`s that makes the trait be able to be used as a service.
///
/// It takes three arguments optionally
/// - `serde_format = _` - Specify a type that implements `trait SerdeFormat`. The default is [serde_cbor](https://github.com/pyfisch/cbor)
/// - `no_proxy` - If provided, the trait will be used only as a service object.
/// - `no_skeleton` - If provided, the trait will be used only as a proxy object.
///
/// There will be many new public `struct`s, but you don't have to know about them.
#[proc_macro_attribute]
pub fn service(args: TokenStream, input: TokenStream) -> TokenStream {
    match service::service(TokenStream2::from(args), TokenStream2::from(input)) {
        Ok(x) => TokenStream::from(x),
        Err(x) => TokenStream::from(x),
    }
}

/// This macro consumes the target trait, and will print the expanded code. Use this when you want to see the result of macro.
#[proc_macro_attribute]
pub fn service_debug(args: TokenStream, input: TokenStream) -> TokenStream {
    match service::service(TokenStream2::from(args), TokenStream2::from(input)) {
        Ok(x) => println!("{}", x),
        Err(x) => println!("{}", x),
    }
    TokenStream::new()
}
