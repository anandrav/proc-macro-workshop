use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_macro_input, Data, DeriveInput, Expr, GenericArgument, Lit, LitStr, Meta, PathArguments,
    Type,
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
        .map(|g| {
            quote! {
                #g,
            }
        })
        .collect();

    let generics_w_trait_bound: Vec<TokenStream> = input
        .generics
        .params
        .iter()
        .map(|g| {
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
            if needs_trait_bound {
                quote! {
                    #g : std::fmt::Debug,
                }
            } else {
                quote! {
                    #g,
                }
            }
        })
        .collect();

    let ret = quote! {
        impl<#( #generics_w_trait_bound )*> std::fmt::Debug for #name<#( #generics )*> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!(#name))
                    #( #field_fmt )*
                   .finish()
            }
        }
    };
    ret.into()
}
