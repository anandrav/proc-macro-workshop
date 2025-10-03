use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_macro_input, Data, DeriveInput, Expr, GenericArgument, GenericParam, Lit, LitStr, Meta,
    PathArguments, Type,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let mut fields = Vec::new();
    match input.data {
        Data::Struct(ds) => {
            for f in ds.fields {
                fields.push(f);
            }
        }
        Data::Enum(_) => {}
        Data::Union(_) => {}
    }

    let field_fmt: Vec<_> = fields
        .iter()
        .map(|f| {
            let mut s: Option<String> = None;
            for attr in &f.attrs {
                match &attr.meta {
                    Meta::NameValue(val) => match &val.value {
                        Expr::Lit(exprlit) => match &exprlit.lit {
                            Lit::Str(lit) => s = Some(lit.value()),
                            _ => todo!("report an error"),
                        },
                        _ => todo!("report an error"),
                    },
                    _ => {}
                }
            }

            let field_name = &f.ident;

            match s {
                None => {
                    quote! {
                        .field(stringify!(#field_name), &self.#field_name)
                    }
                }
                Some(s) => {
                    quote! {
                        .field(stringify!(#field_name), &format_args!(#s, &self.#field_name))
                    }
                }
            }
        })
        .collect();

    let generics: Vec<TokenStream> = input
        .generics
        .params
        .iter()
        .filter_map(|g| {
            if let GenericParam::Type(t) = g {
                let ident = &t.ident;
                Some(quote! {
                    #ident
                })
            } else {
                None
            }
        })
        .collect();

    let generics_w_trait_bound: Vec<TokenStream> = input
        .generics
        .params
        .iter()
        .filter_map(|g| {
            let mut needs_trait_bound = true;
            for field in &fields {
                let field_ty_s = field.ty.to_token_stream().to_string();
                if field_ty_s.starts_with("PhantomData") {
                    let Some(first_delim) = field_ty_s.find('<') else {
                        continue;
                    };
                    let Some(second_delim) = field_ty_s.find('>') else {
                        continue;
                    };
                    let inner = field_ty_s[first_delim + 1..second_delim].trim();
                    if g.to_token_stream().to_string() == inner {
                        needs_trait_bound = false;
                    }
                }
            }
            let GenericParam::Type(t) = g else {
                return None;
            };
            // let ident = &t.ident;
            Some(quote! {
                #t
            })
            // Some(if needs_trait_bound {
            //     quote! {
            //         #ident : std::fmt::Debug,
            //     }
            // } else {
            //     quote! {
            //         #ident,
            //     }
            // })
        })
        .collect();

    let mut generics_w_debug_bound = Vec::new();
    for param in input.generics.params.iter() {
        if let GenericParam::Type(t) = param {
            // is this type param mentioned in any of the fields? (other than PhantomData)
            if fields
                .iter()
                .any(|f| ty_mentions_generic_param(&f.ty, &t.ident))
            {
                let ident = &t.ident;
                generics_w_debug_bound.push(quote! {
                    #ident : std::fmt::Debug,
                });
            }
        }
    }
    // let generics_w_debug_bound: Vec<TokenStream> = input
    //     .generics
    //     .params
    //     .iter()
    //     .filter_map(|g| {
    //         let mut needs_trait_bound = false;
    //         for field in &fields {
    //             // let field_ty_s = field.ty.to_token_stream().to_string();
    //             // if field_ty_s.starts_with("PhantomData") {
    //             //     let Some(first_delim) = field_ty_s.find('<') else {
    //             //         continue;
    //             //     };
    //             //     let Some(second_delim) = field_ty_s.find('>') else {
    //             //         continue;
    //             //     };
    //             //     let inner = field_ty_s[first_delim + 1..second_delim].trim();
    //             //     if g.to_token_stream().to_string() == inner {
    //             //         needs_trait_bound = false;
    //             //     }
    //             // }
    //         }
    //         if !needs_trait_bound {
    //             return None;
    //         }
    //         let GenericParam::Type(t) = g else {
    //             return None;
    //         };
    //         let ident = &t.ident;
    //         Some(quote! {
    //             #ident : std::fmt::Debug,
    //         })
    //     })
    //     .collect();

    // Collect type parameter idents
    let type_param_idents: Vec<Ident> = input
        .generics
        .params
        .iter()
        .filter_map(|p| match p {
            GenericParam::Type(ty) => Some(ty.ident.clone()),
            _ => None,
        })
        .collect();

    let mut assoc_types = Vec::new();
    for field in &fields {
        get_associated_types(&field.ty, &mut assoc_types, &type_param_idents);
    }

    dbg!(type_param_idents);

    let assoc_typ_trait_bounds = assoc_types
        .iter()
        .map(|t| {
            quote! {
                #t : std::fmt::Debug,
            }
        })
        .collect::<Vec<_>>();
    //
    // let assoc_typ_trait_bounds: Vec<TokenStream> = fields
    //     .iter()
    //     .filter_map(|f| {
    //         if let Type::Path(tp) = &f.ty {
    //             panic!("{}", &f.ty.to_token_stream().to_string());
    //             let Some(first_segment) = tp.path.segments.first() else {
    //                 return None;
    //             };
    //             if tp.path.segments.len() < 2 {
    //                 return None;
    //             }
    //             if input
    //                 .generics
    //                 .params
    //                 .iter()
    //                 .find(|p| {
    //                     panic!(
    //                         "p={}, first seg={}",
    //                         p.to_token_stream(),
    //                         first_segment.ident
    //                     );
    //                     p.to_token_stream().to_string() == first_segment.ident.to_string()
    //                 })
    //                 .is_some()
    //             {
    //                 return Some(&f.ty);
    //             }
    //         }
    //         None
    //     })
    //     .map(|t| {
    //         panic!("{}", t.to_token_stream());
    //         quote! {
    //             #t : std::fmt::Debug,
    //         }
    //     })
    //     .collect();
    // assert!(!assoc_typ_trait_bounds.is_empty());

    let ret = quote! {
        impl<#( #generics_w_trait_bound )*> std::fmt::Debug for #name<#( #generics )*>
        where #( #assoc_typ_trait_bounds )* #( #generics_w_debug_bound )*
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!(#name))
                    #( #field_fmt )*
                   .finish()
            }
        }
    };
    eprintln!("TOKENS: {}", ret);
    ret.into()
}

fn ty_mentions_generic_param(ty: &Type, param: &Ident) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(last_seg) = type_path.path.segments.last() {
            if last_seg.ident != "PhantomData" {
                if let PathArguments::AngleBracketed(args) = &last_seg.arguments {
                    for arg in args.args.iter() {
                        if let GenericArgument::Type(ty) = arg {
                            if ty_mentions_generic_param(ty, param) {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        if type_path.path.segments.len() == 1 {
            let first_seg = type_path.path.segments.first().unwrap();
            if first_seg.ident == param.to_string() {
                return true;
            }
        }
    }
    return false;
}

fn get_associated_types(ty: &Type, associated_types: &mut Vec<Type>, type_params: &[Ident]) {
    if let Type::Path(type_path) = ty {
        if let Some(last_seg) = type_path.path.segments.last() {
            if let PathArguments::AngleBracketed(args) = &last_seg.arguments {
                for arg in args.args.iter() {
                    if let GenericArgument::Type(ty) = arg {
                        get_associated_types(ty, associated_types, type_params);
                    }
                }
            }
        }

        if type_path.path.segments.len() > 1 {
            if let Some(first_seg) = type_path.path.segments.first() {
                if type_params.contains(&first_seg.ident) {
                    associated_types.push(ty.clone());
                }
            }
        }
    }
}
