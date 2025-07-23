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

    let field_name: Vec<_> = fields.iter().map(|f| &f.ident).collect();

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

    let ret = quote! {
        impl std::fmt::Debug for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!(#name))
                    #( #field_fmt )*
                   .finish()
            }
        }
    };
    ret.into()
}
