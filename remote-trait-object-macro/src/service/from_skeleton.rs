use super::MacroArgs;
use crate::create_env_path;
use crate::helper::path_of_single_ident;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::ToTokens;

pub(super) fn generate_from_skeleton(
    source_trait: &syn::ItemTrait,
    args: &MacroArgs,
) -> Result<TokenStream2, TokenStream2> {
    if args.no_proxy {
        return Ok(TokenStream2::new());
    }

    let env_path = create_env_path();

    let trait_ident = source_trait.ident.clone();
    let struct_ident = quote::format_ident!("{}FromSkeleton", trait_ident);
    let mut imported_struct = quote! {
        #[derive(Debug)]
        #[doc(hidden)]
        /// This type is generated by the remote-trait-object macro.
        /// It should never be used directly by you, so please ignore it.
        pub struct #struct_ident {
            skeleton: #env_path::Skeleton
        }
    };
    let mut imported_struct_impl = syn::parse2::<syn::ItemImpl>(quote! {
        impl #trait_ident for #struct_ident {
        }
    })
    .unwrap();
    let serde_format = &args.serde_format;

    for item in source_trait.items.iter() {
        let method = match item {
            syn::TraitItem::Method(x) => x,
            non_method => {
                return Err(syn::Error::new_spanned(
                    non_method,
                    "Service trait must have only methods",
                )
                .to_compile_error())
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
                        arguments_in_tuple
                            .elems
                            .push(syn::Expr::Path(syn::ExprPath {
                                attrs: Vec::new(),
                                qself: None,
                                path: path_of_single_ident(the_arg.ident.clone()),
                            }));
                    } else {
                        return Err(syn::Error::new_spanned(
                            arg,
                            "You must not use a pattern for the argument",
                        )
                        .to_compile_error());
                    }
                }
            }
        }

        let the_call = quote! {
            let args = <#serde_format as #env_path::SerdeFormat>::to_vec(&#arguments_in_tuple).unwrap();
            let result = #env_path::get_dispatch(&self.skeleton).dispatch_and_call(#id_ident.load(#env_path::ID_ORDERING), &args);
            <#serde_format as #env_path::SerdeFormat>::from_slice(&result).unwrap()
        };
        the_method
            .block
            .stmts
            .push(syn::Stmt::Expr(syn::Expr::Verbatim(the_call)));
        imported_struct_impl
            .items
            .push(syn::ImplItem::Method(the_method));
    }
    imported_struct.extend(imported_struct_impl.to_token_stream());
    imported_struct.extend(quote! {
        impl #env_path::Service for #struct_ident {
        }
        impl #env_path::FromSkeleton<dyn #trait_ident> for Box<dyn #trait_ident> {
            fn from_skeleton(skeleton: #env_path::Skeleton) -> Self {
                Box::new(#struct_ident {
                    skeleton
                })
            }
        }
        impl #env_path::FromSkeleton<dyn #trait_ident> for std::sync::Arc<dyn #trait_ident> {
            fn from_skeleton(skeleton: #env_path::Skeleton) -> Self {
                std::sync::Arc::new(#struct_ident {
                    skeleton
                })
            }
        }
        impl #env_path::FromSkeleton<dyn #trait_ident> for std::sync::Arc<parking_lot::RwLock<dyn #trait_ident>> {
            fn from_skeleton(skeleton: #env_path::Skeleton) -> Self {
                std::sync::Arc::new(parking_lot::RwLock::new(#struct_ident {
                    skeleton
                }))
            }
        }
    });
    Ok(imported_struct.to_token_stream())
}
