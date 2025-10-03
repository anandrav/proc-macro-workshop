use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::{
    parse_macro_input, Data, DeriveInput, Expr, GenericArgument, GenericParam, Lit, Meta,
    PathArguments, Type,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let mut bound_attr = TokenStream::new();
    let mut infer_debug_bounds = true;

    for attr in &input.attrs {
        if attr.path().is_ident("debug") {
            if let syn::Meta::List(list) = &attr.meta {
                // In syn v2, list contains only path + tokens
                let args: syn::punctuated::Punctuated<syn::MetaNameValue, syn::Token![,]> = list
                    .parse_args_with(syn::punctuated::Punctuated::parse_terminated)
                    .unwrap();

                for nv in args {
                    if nv.path.is_ident("bound") {
                        // panic!("HERE");
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(s),
                            ..
                        }) = nv.value
                        {
                            println!("bound = {}", s.value());
                            bound_attr = TokenStream::from_str(&s.value()).unwrap();
                            infer_debug_bounds = false;
                        }
                    }
                }
            }
        }
    }

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
                if let Meta::NameValue(val) = &attr.meta {
                    match &val.value {
                        Expr::Lit(exprlit) => match &exprlit.lit {
                            Lit::Str(lit) => s = Some(lit.value()),
                            _ => todo!("report an error"),
                        },
                        _ => todo!("report an error"),
                    }
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
            let GenericParam::Type(t) = g else {
                return None;
            };
            Some(quote! {
                #t
            })
        })
        .collect();

    let mut generics_w_debug_bound = Vec::new();
    if infer_debug_bounds {
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
    }

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

    let ret = quote! {
        impl<#( #generics_w_trait_bound )*> std::fmt::Debug for #name<#( #generics )*>
        where #( #assoc_typ_trait_bounds )* #( #generics_w_debug_bound )* #bound_attr
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
            if first_seg.ident == *param {
                return true;
            }
        }
    }
    false
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
