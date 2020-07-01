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
    let box_dispatcher_ident = quote::format_ident!("{}BoxDispatcher", trait_ident);
    let arc_dispatcher_ident = quote::format_ident!("{}ArcDispatcher", trait_ident);
    let rwlock_dispatcher_ident = quote::format_ident!("{}RwLockDispatcher", trait_ident);

    // TODO: If # of methods is larger than certain limit,
    // then introduce a closure list for the method dispatch,
    // instead of if-else clauses
    let mut if_else_clauses = TokenStream2::new();
    let mut if_else_clauses_rwlock = TokenStream2::new();

    let mut is_this_trait_mutable = false;

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

        let no_self = "All your method must take &self or &mut self (Object safety)";
        let mut_self = match method
            .sig
            .inputs
            .first()
            .ok_or_else(|| syn::Error::new_spanned(method, no_self).to_compile_error())?
        {
            syn::FnArg::Typed(_) => return Err(syn::Error::new_spanned(method, no_self).to_compile_error()),
            syn::FnArg::Receiver(syn::Receiver {
                mutability: Some(_),
                ..
            }) => true,
            _ => false,
        };
        is_this_trait_mutable |= mut_self;

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
        let stmt_call_rwlock = if mut_self {
            quote! {
                let result = self.object.write().#method_name(#the_args);
            }
        } else {
            quote! {
                let result = self.object.read().#method_name(#the_args);
            }
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

        if_else_clauses_rwlock.extend(quote! {
            if method == #id_ident.load(#env_path::ID_ORDERING) {
                #stmt_deserialize
                #stmt_call_rwlock
                #the_return
            }
        });
    }
    if_else_clauses.extend(quote! {
        panic!("Invalid remote-trait-object call. Fatal Error.")
    });
    if_else_clauses_rwlock.extend(quote! {
        panic!("Invalid remote-trait-object call. Fatal Error.")
    });

    let box_dispatcher = if is_this_trait_mutable {
        quote! {
            pub struct #box_dispatcher_ident {
                object: parking_lot::RwLock<Box<dyn #trait_ident>>
            }
            impl #box_dispatcher_ident {
                fn new(object: Box<dyn #trait_ident>) -> Self {
                    Self {
                        object: parking_lot::RwLock::new(object)
                    }
                }
            }
            impl #env_path::Dispatch for #box_dispatcher_ident {
                fn dispatch_and_call(&self, method: #env_path::MethodId, args: &[u8]) -> Vec<u8> {
                    #if_else_clauses_rwlock
                }
            }
            impl #env_path::ToDispatcher<dyn #trait_ident> for Box<dyn #trait_ident> {
                fn to_dispatcher(self) -> std::sync::Arc<dyn #env_path::Dispatch> {
                    std::sync::Arc::new(#box_dispatcher_ident::new(self))
                }
            }
        }
    } else {
        quote! {
            pub struct #box_dispatcher_ident {
                object: Box<dyn #trait_ident>
            }
            impl #box_dispatcher_ident {
                fn new(object: Box<dyn #trait_ident>) -> Self {
                    Self {
                        object
                    }
                }
            }
            impl #env_path::Dispatch for #box_dispatcher_ident {
                fn dispatch_and_call(&self, method: #env_path::MethodId, args: &[u8]) -> Vec<u8> {
                    #if_else_clauses
                }
            }
            impl #env_path::ToDispatcher<dyn #trait_ident> for Box<dyn #trait_ident> {
                fn to_dispatcher(self) -> std::sync::Arc<dyn #env_path::Dispatch> {
                    std::sync::Arc::new(#box_dispatcher_ident::new(self))
                }
            }
        }
    };

    let arc_dispatcher = if is_this_trait_mutable {
        quote! {}
    } else {
        quote! {
            pub struct #arc_dispatcher_ident {
                object: std::sync::Arc<dyn #trait_ident>
            }
            impl #arc_dispatcher_ident {
                fn new(object: std::sync::Arc<dyn #trait_ident>) -> Self {
                    Self {
                        object
                    }
                }
            }
            impl #env_path::Dispatch for #arc_dispatcher_ident {
                fn dispatch_and_call(&self, method: #env_path::MethodId, args: &[u8]) -> Vec<u8> {
                    #if_else_clauses
                }
            }
            impl #env_path::ToDispatcher<dyn #trait_ident> for std::sync::Arc<dyn #trait_ident> {
                fn to_dispatcher(self) -> std::sync::Arc<dyn #env_path::Dispatch> {
                    std::sync::Arc::new(#arc_dispatcher_ident::new(self))
                }
            }
        }
    };

    let rwlock_dispatcher = quote! {
        pub struct #rwlock_dispatcher_ident {
            object: std::sync::Arc<parking_lot::RwLock<dyn #trait_ident>>
        }
        impl #rwlock_dispatcher_ident {
            fn new(object: std::sync::Arc<parking_lot::RwLock<dyn #trait_ident>>) -> Self {
                Self {
                    object
                }
            }
        }
        impl #env_path::Dispatch for #rwlock_dispatcher_ident {
            fn dispatch_and_call(&self, method: #env_path::MethodId, args: &[u8]) -> Vec<u8> {
                #if_else_clauses_rwlock
            }
        }
        impl #env_path::ToDispatcher<dyn #trait_ident> for std::sync::Arc<parking_lot::RwLock<dyn #trait_ident>> {
            fn to_dispatcher(self) -> std::sync::Arc<dyn #env_path::Dispatch> {
                std::sync::Arc::new(#rwlock_dispatcher_ident::new(self))
            }
        }
    };

    Ok(quote! {
        #box_dispatcher
        #arc_dispatcher
        #rwlock_dispatcher
    })
}
