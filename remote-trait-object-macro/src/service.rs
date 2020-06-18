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

use crate::helper;
use proc_macro2::TokenStream as TokenStream2;

// TODOs - currently just a identity function
// 1. Implement ID registeration
// 2. Implement dispatcher
// 3. Implement remote
// 4. Implement export / import
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

    let id = helper::id::generate_id(&source_trait)?;
    let dispatcher = helper::dispatcher::generate_dispatcher(&source_trait)?;
    let remote = helper::remote::generate_remote(&source_trait)?;
    let export_and_import = helper::export_import::generate_export_and_import(&source_trait)?;

    Ok(quote! {
        #source_trait
        #id
        #dispatcher
        #remote
        #export_and_import
    })
}
