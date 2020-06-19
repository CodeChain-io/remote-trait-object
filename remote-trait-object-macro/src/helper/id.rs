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

use crate::create_env_path;
use proc_macro2::{Span, TokenStream as TokenStream2};
use syn::Ident;

pub fn id_method_ident(the_trait: &syn::ItemTrait, method: &syn::TraitItemMethod) -> Ident {
    quote::format_ident!("ID_METHOD_{}_{}", the_trait.ident, method.sig.ident)
}

fn lit_index(index: usize) -> syn::Lit {
    // We put a distinctive offset for the easy debug.
    syn::Lit::Int(syn::LitInt::new(&format!("{}", index + 70), Span::call_site()))
}

fn id_method_entry_ident(the_trait: &syn::ItemTrait, method: &syn::TraitItemMethod) -> Ident {
    quote::format_ident!("ID_METHOD_ENTRY_{}_{}", the_trait.ident, method.sig.ident)
}

fn id_method_setter_ident(the_trait: &syn::ItemTrait, method: &syn::TraitItemMethod) -> Ident {
    quote::format_ident!("id_method_setter_{}_{}", the_trait.ident, method.sig.ident)
}

pub fn generate_id(source_trait: &syn::ItemTrait) -> Result<TokenStream2, TokenStream2> {
    let env_path = create_env_path();
    let lit_trait_name = syn::LitStr::new(&format!("{}", source_trait.ident), Span::call_site());
    let mut method_id_table = TokenStream2::new();

    for (i, item) in source_trait.items.iter().enumerate() {
        let method = match item {
            syn::TraitItem::Method(x) => x,
            non_method => {
                return Err(
                    syn::Error::new_spanned(non_method, "Service trait must have only methods").to_compile_error()
                )
            }
        };
        let lit_index = lit_index(i);
        let lit_method_name = syn::LitStr::new(&format!("{}", method.sig.ident), Span::call_site());

        let id_ident = id_method_ident(&source_trait, method);
        let id_entry_ident = id_method_entry_ident(&source_trait, method);
        let id_setter_ident = id_method_setter_ident(&source_trait, method);
        let id_entry = quote! {
            #[allow(non_upper_case_globals)]
            static #id_ident: #env_path::MethodIdAtomic = #env_path::MethodIdAtomic::new(#lit_index);
            #[linkme::distributed_slice(#env_path::MID_REG)]
            #[allow(non_upper_case_globals)]
            static #id_entry_ident: (&'static str, &'static str, fn(id: #env_path::MethodId)) =
            (#lit_trait_name, #lit_method_name, #id_setter_ident);
            #[allow(non_snake_case)]
            fn #id_setter_ident(id: #env_path::MethodId) {
                #id_ident.store(id, #env_path::ID_ORDERING);
            }
        };
        method_id_table.extend(id_entry);
    }
    Ok(method_id_table)
}
