pub mod dispatcher;
pub mod export_import;
pub mod id;
pub mod remote;

pub fn path_of_single_ident(ident: syn::Ident) -> syn::Path {
    syn::Path {
        leading_colon: None,
        segments: {
            let mut punc = syn::punctuated::Punctuated::new();
            punc.push(syn::PathSegment {
                ident,
                arguments: syn::PathArguments::None,
            });
            punc
        },
    }
}
