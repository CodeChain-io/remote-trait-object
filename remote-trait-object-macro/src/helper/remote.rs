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

use super::path_of_single_ident;
use crate::create_env_path;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::ToTokens;

pub fn generate_remote(source_trait: &syn::ItemTrait) -> Result<TokenStream2, TokenStream2> {
    let env_path = create_env_path();

    let trait_ident = source_trait.ident.clone();
    let struct_ident = quote::format_ident!("{}Remote", trait_ident);
    let mut imported_struct = quote! {
        #[derive(Debug)]
        pub struct #struct_ident {
            handle: #env_path::Handle
        }
    };
    let mut imported_struct_impl = syn::parse2::<syn::ItemImpl>(quote! {
        impl #trait_ident for #struct_ident {
        }
    })
    .unwrap();

    for item in source_trait.items.iter() {
        let method = match item {
            syn::TraitItem::Method(x) => x,
            non_method => {
                return Err(
                    syn::Error::new_spanned(non_method, "Service trait must have only methods").to_compile_error()
                )
            }
        };
        let id_ident = super::id::id_method_ident(source_trait, method);

        let mut the_method = syn::parse_str::<syn::ImplItemMethod>("fn dummy() -> () {}").unwrap();
        the_method.sig = method.sig.clone();
        let mut arguments_in_tuple = syn::ExprTuple {
            attrs: Vec::new(),
            paren_token: syn::token::Paren(Span::call_site()),
            elems: syn::punctuated::Punctuated::new(),
        };
        for arg in &method.sig.inputs {
            match arg {
                syn::FnArg::Receiver(_) => continue, // &self
                syn::FnArg::Typed(pattern) => {
                    if let syn::Pat::Ident(the_arg) = &*pattern.pat {
                        arguments_in_tuple.elems.push(syn::Expr::Path(syn::ExprPath {
                            attrs: Vec::new(),
                            qself: None,
                            path: path_of_single_ident(the_arg.ident.clone()),
                        }));
                    } else {
                        return Err(syn::Error::new_spanned(arg, "You must not use a pattern for the argument")
                            .to_compile_error())
                    }
                }
            }
        }

        let the_call = quote! {
            self.handle.call(#id_ident.load(#env_path::ID_ORDERING), &#arguments_in_tuple)
        };
        the_method.block.stmts.push(syn::Stmt::Expr(syn::Expr::Verbatim(the_call)));
        imported_struct_impl.items.push(syn::ImplItem::Method(the_method));
    }
    imported_struct.extend(imported_struct_impl.to_token_stream());
    imported_struct.extend(quote! {
        impl #env_path::Service for #struct_ident {
        }
        impl #env_path::ToRemote<dyn #trait_ident> for Box<dyn #trait_ident> {
            fn to_remote(port: std::sync::Weak<dyn #env_path::Port>, handle: #env_path::HandleToExchange) -> Self {
                Box::new(#struct_ident {
                    handle: #env_path::Handle::careful_new(handle, port),
                })
            }
        }
        impl #env_path::ToRemote<dyn #trait_ident> for std::sync::Arc<dyn #trait_ident> {
            fn to_remote(port: std::sync::Weak<dyn #env_path::Port>, handle: #env_path::HandleToExchange) -> Self {
                std::sync::Arc::new(#struct_ident {
                    handle: #env_path::Handle::careful_new(handle, port),
                })
            }
        }
        impl #env_path::ToRemote<dyn #trait_ident> for std::sync::Arc<parking_lot::RwLock<dyn #trait_ident>> {
            fn to_remote(port: std::sync::Weak<dyn #env_path::Port>, handle: #env_path::HandleToExchange) -> Self {
                std::sync::Arc::new(parking_lot::RwLock::new(#struct_ident {
                    handle: #env_path::Handle::careful_new(handle, port),
                }))
            }
        }
    });
    Ok(imported_struct.to_token_stream())
}
