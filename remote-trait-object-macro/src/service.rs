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
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::Token;

pub mod dispatcher;
pub mod id;
pub mod proxy;

struct SingleArg<T: Parse> {
    pub arg_name: syn::Ident,
    pub arg_value: T,
}

impl<T: Parse> Parse for SingleArg<T> {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let arg_name = input.parse()?;
        input.parse::<Token![=]>()?;
        let arg_value = input.parse()?;
        Ok(Self {
            arg_name,
            arg_value,
        })
    }
}

#[derive(Default)]
struct MacroArgsRaw {
    pub serde_format: Option<syn::Path>,
    pub no_proxy: Option<()>,
    pub no_skeleton: Option<()>,
}

struct MacroArgs {
    pub serde_format: syn::Path,
    pub no_proxy: bool,
    pub no_skeleton: bool,
}

impl MacroArgsRaw {
    fn update(&mut self, ts: TokenStream2) -> syn::parse::Result<()> {
        if let Ok(arg) = syn::parse2::<syn::Ident>(ts.clone()) {
            return if arg == quote::format_ident!("no_proxy") {
                if self.no_proxy.replace(()).is_some() {
                    Err(syn::parse::Error::new_spanned(ts, "Duplicated arguments"))
                } else {
                    Ok(())
                }
            } else if arg == quote::format_ident!("no_skeleton") {
                if self.no_skeleton.replace(()).is_some() {
                    Err(syn::parse::Error::new_spanned(ts, "Duplicated arguments"))
                } else {
                    Ok(())
                }
            } else {
                Err(syn::parse::Error::new_spanned(ts, "Unsupported argument"))
            }
        }

        let arg: SingleArg<TokenStream2> = syn::parse2(ts.clone())?;
        if arg.arg_name == quote::format_ident!("serde_format") {
            let value = syn::parse2(arg.arg_value)?;
            if self.serde_format.replace(value).is_some() {
                Err(syn::parse::Error::new_spanned(ts, "Duplicated arguments"))
            } else {
                Ok(())
            }
        } else {
            Err(syn::parse::Error::new_spanned(ts, "Unsupported argument"))
        }
    }

    fn fill_default_values(self) -> MacroArgs {
        MacroArgs {
            serde_format: self
                .serde_format
                .unwrap_or_else(|| syn::parse2(quote! {remote_trait_object::macro_env::DefaultSerdeFormat}).unwrap()),
            no_proxy: self.no_proxy.map(|_| true).unwrap_or(false),
            no_skeleton: self.no_skeleton.map(|_| true).unwrap_or(false),
        }
    }
}

impl Parse for MacroArgsRaw {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let mut result = MacroArgsRaw::default();
        let args = Punctuated::<syn::Expr, Token![,]>::parse_terminated(input)?;
        for arg in args {
            result.update(quote! {#arg})?;
        }
        Ok(result)
    }
}

pub fn service(args: TokenStream2, input: TokenStream2) -> Result<TokenStream2, TokenStream2> {
    let args: MacroArgsRaw = syn::parse2(args).map_err(|e| e.to_compile_error())?;
    let args = args.fill_default_values();

    let source_trait = match syn::parse2::<syn::ItemTrait>(input.clone()) {
        Ok(x) => x,
        Err(_) => {
            return Err(syn::Error::new_spanned(input, "You can use #[service] only on a trait").to_compile_error())
        }
    };

    let id = id::generate_id(&source_trait, &args)?;
    let dispatcher = dispatcher::generate_dispatcher(&source_trait, &args)?;
    let proxy = proxy::generate_proxy(&source_trait, &args)?;

    Ok(quote! {
        #source_trait
        #id
        #dispatcher
        #proxy
    })
}
