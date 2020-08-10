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

/// In addition, it coverts str->String and [] -> Vec
pub fn is_ref(the_type: &syn::Type) -> Result<Option<syn::Type>, String> {
    if *the_type
        == syn::parse2::<syn::Type>(quote! {
            &str
        })
        .unwrap()
    {
        return Ok(Some(
            syn::parse2::<syn::Type>(quote! {
                String
            })
            .unwrap(),
        ))
    }

    match the_type {
        syn::Type::Reference(x) => {
            if x.lifetime.is_some() {
                return Err("Lifetime exists".to_owned())
            }
            if x.mutability.is_some() {
                return Err("Mutable".to_owned())
            }
            match *x.elem {
                syn::Type::Slice(_) => Ok(Some(
                    syn::parse2::<syn::Type>(quote! {
                        Vec<_>
                    })
                    .unwrap(),
                )),
                _ => Ok(Some((*x.elem).clone())),
            }
        }
        _ => Ok(None),
    }
}

#[test]
fn recognize_ref() {
    let t = syn::parse_str::<syn::Type>("Vec<u32>").unwrap();
    assert!(is_ref(&t).unwrap().is_none());
    let t = syn::parse_str::<syn::Type>("&Vec<u32>").unwrap();
    let tu = syn::parse_str::<syn::Type>("Vec<u32>").unwrap();
    assert_eq!(is_ref(&t).unwrap().unwrap(), tu);
    let t = syn::parse_str::<syn::Type>("&i32").unwrap();
    let tu = syn::parse_str::<syn::Type>("i32").unwrap();
    assert_eq!(is_ref(&t).unwrap().unwrap(), tu);
    let t = syn::parse_str::<syn::Type>("&str").unwrap();
    let tu = syn::parse_str::<syn::Type>("String").unwrap();
    assert_eq!(is_ref(&t).unwrap().unwrap(), tu);
    let t = syn::parse_str::<syn::Type>("&[u8]").unwrap();
    let tu = syn::parse_str::<syn::Type>("Vec<_>").unwrap();
    assert_eq!(is_ref(&t).unwrap().unwrap(), tu);
    let t = syn::parse_str::<syn::Type>("&mut i32").unwrap();
    assert!(is_ref(&t).is_err())
}
