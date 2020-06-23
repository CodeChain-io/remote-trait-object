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

pub fn generate_dispatcher(source_trait: &syn::ItemTrait) -> Result<TokenStream2, TokenStream2> {
    let env_path = create_env_path();
    let trait_ident = source_trait.ident.clone();
    let struct_ident = quote::format_ident!("{}Dispatcher", trait_ident);

    // TODO: If # of methods is larger than certain limit,
    // then introduce a closure list for the method dispatch,
    // instead of if-else clauses
    let mut if_else_clauses = TokenStream2::new();

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

        // Argument will be represented as a tuple. We deserialize the data as a tuple here
        let mut the_let_pattern = syn::PatTuple {
            attrs: Vec::new(),
            paren_token: syn::token::Paren(Span::call_site()),
            elems: syn::punctuated::Punctuated::new(),
        };
        // They need annotation
        let mut type_annotation = syn::TypeTuple {
            paren_token: syn::token::Paren(Span::call_site()),
            elems: syn::punctuated::Punctuated::new(),
        };
        // We apply the arguments on the designated method, performing an actuall call.
        let mut the_args: syn::punctuated::Punctuated<syn::Expr, syn::token::Comma> =
            syn::punctuated::Punctuated::new();

        let no_self = "All your method must take &self";
        if let syn::FnArg::Typed(_) =
            method.sig.inputs.first().ok_or_else(|| syn::Error::new_spanned(method, no_self).to_compile_error())?
        {
            return Err(syn::Error::new_spanned(method, no_self).to_compile_error())
        }

        for (j, arg_source) in method.sig.inputs.iter().skip(1).enumerate() {
            let the_iden = quote::format_ident!("a{}", j + 1);
            the_let_pattern.elems.push(syn::Pat::Ident(syn::PatIdent {
                attrs: Vec::new(),
                by_ref: None,
                mutability: None,
                ident: the_iden,
                subpat: None,
            }));
            the_let_pattern.elems.push_punct(syn::token::Comma(Span::call_site()));

            let arg_type = match arg_source {
                syn::FnArg::Typed(syn::PatType {
                    attrs: _,
                    pat: _,
                    colon_token: _,
                    ty: t,
                }) => &**t,
                _ => panic!(),
            };

            if let Some(unrefed_type) = super::types::is_ref(arg_type)
                .map_err(|e| syn::Error::new_spanned(arg_source, &e).to_compile_error())?
            {
                type_annotation.elems.push(unrefed_type);
            } else {
                type_annotation.elems.push(arg_type.clone());
            }

            type_annotation.elems.push_punct(syn::token::Comma(Span::call_site()));

            let arg_ident = quote::format_ident!("a{}", j + 1);
            let the_arg = if super::types::is_ref(arg_type)
                .map_err(|e| syn::Error::new_spanned(arg_source, &e).to_compile_error())?
                .is_some()
            {
                quote! {
                    &#arg_ident
                }
            } else {
                quote! {
                    #arg_ident
                }
            };
            the_args.push(syn::parse2(the_arg).unwrap());
        }

        let stmt_deserialize = quote! {
            // TODO: Make the macro be able to take deserialization scheme
            let #the_let_pattern: #type_annotation = serde_cbor::from_slice(args).unwrap();
        };

        let method_name = method.sig.ident.clone();
        let stmt_call = quote! {
            let result = self.object.#method_name(#the_args);
        };

        let the_return = quote! {
            return serde_cbor::to_vec(&result).unwrap();
        };

        if_else_clauses.extend(quote! {
            if method == #id_ident.load(#env_path::ID_ORDERING) {
                #stmt_deserialize
                #stmt_call
                #the_return
            }
        });
    }
    if_else_clauses.extend(quote! {
        panic!("Invalid remote-trait-object call. Fatal Error.")
    });

    Ok(quote! {
        pub struct #struct_ident {
            object: std::sync::Arc<dyn #trait_ident>
        }
        impl #struct_ident {
            pub fn new(object: std::sync::Arc<dyn #trait_ident>) -> Self {
                Self {
                    object
                }
            }
        }
        impl #env_path::Dispatch for #struct_ident {
            fn dispatch_and_call(&self, method: #env_path::MethodId, args: &[u8]) -> Vec<u8> {
                #if_else_clauses
            }
        }
        impl #env_path::ExportService<dyn #trait_ident> for dyn #trait_ident {
            fn export(port: std::sync::Weak<dyn #env_path::Port>, object: std::sync::Arc<dyn #trait_ident>) -> #env_path::HandleToExchange {
                port.upgrade().unwrap().register(std::sync::Arc::new(#struct_ident::new(object)))
            }
        }
    })
}
