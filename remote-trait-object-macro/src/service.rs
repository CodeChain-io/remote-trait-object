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

use proc_macro2::TokenStream as TokenStream2;

pub mod dispatcher;
pub mod id;
pub mod remote;

pub fn service(args: TokenStream2, input: TokenStream2) -> Result<TokenStream2, TokenStream2> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(input, "#[service] doesn't take any argument").to_compile_error())
    }

    let source_trait = match syn::parse2::<syn::ItemTrait>(input.clone()) {
        Ok(x) => x,
        Err(_) => {
            return Err(syn::Error::new_spanned(input, "You can use #[service] only on a trait").to_compile_error())
        }
    };

    let id = id::generate_id(&source_trait)?;
    let dispatcher = dispatcher::generate_dispatcher(&source_trait)?;
    let remote = remote::generate_remote(&source_trait)?;

    Ok(quote! {
        #source_trait
        #id
        #dispatcher
        #remote
    })
}
