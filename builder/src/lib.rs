use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, GenericArgument, LitStr, PathArguments, Type};

#[proc_macro_derive(Builder, attributes(builder))]
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

    let builder_name = format_ident!("{}Builder", name);

    let field_name: Vec<_> = fields.iter().map(|f| &f.ident).collect();

    let is_optional = |t: &Type| get_inner_ty(t, "Option").is_some();

    let is_vec = |t: &Type| get_inner_ty(t, "Vec").is_some();

    let field_conversion: Vec<TokenStream> = fields.iter().map(|f| {
        let field_name = &f.ident.as_ref().unwrap();
        if is_optional(&f.ty) || is_vec(&f.ty) {
            quote! {
                let #field_name = &self.#field_name;
            }
        } else {
            quote! {
                let std::option::Option::Some(#field_name) = &self.#field_name else { return std::result::Result::Err(std::boxed::Box::from(format!("field cannot be None: {}", stringify!(#field_name)).to_string()))};
            }
        }
    }).collect();

    let builder_field: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let field_name = &f.ident.as_ref().unwrap();
            let field_type = &f.ty;
            if is_optional(&f.ty) || is_vec(&f.ty) {
                quote! {
                    #field_name: #field_type,
                }
            } else {
                quote! {
                    #field_name: std::option::Option<#field_type>,
                }
            }
        })
        .collect();

    let field_setter: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let field_name = &f.ident.as_ref().unwrap();
            let opt_inner_ty = get_inner_ty(&f.ty, "Option");
            let field_type = match opt_inner_ty {
                Some(inner_ty) => inner_ty,
                None => &f.ty,
            };
            let mut builder_attr_each = None;
            for attr in &f.attrs {
                if attr.path().is_ident("builder") {
                    let mut value = None;
                    if let Err(e) = attr.parse_nested_meta(|meta| {
                        if meta.path.is_ident("each") {
                            value = Some(meta.value()?.parse::<LitStr>()?);
                            Ok(())
                        } else {
                            Err(meta.error("expected `builder(each = \"...\")`"))
                        }
                    }) {
                        return e.to_compile_error();
                    }
                    let value = value.expect("Could not parse attribute value");
                    let str = value.value();
                    let ident: Ident = Ident::new(&str, value.span());
                    builder_attr_each = Some(ident);
                }
            }
            let vec_inner_ty = get_inner_ty(&f.ty, "Vec");
            let non_vec_setter = || {
                quote! {
                    pub fn #field_name(&mut self, it: #field_type) -> &mut Self {
                        self.#field_name = Some(it);
                        self
                    }
                }
            };
            let vec_setter = || {
                quote! {
                    pub fn #field_name(&mut self, it: #field_type) -> &mut Self {
                        self.#field_name = it;
                        self
                    }
                }
            };
            let default = |t: &Type| {
                if is_vec(t) {
                    vec_setter()
                } else {
                    non_vec_setter()
                }
            };
            match builder_attr_each {
                Some(builder_attr_each) => {
                    if builder_attr_each != **field_name {
                        let Some(field_type) = vec_inner_ty else {
                            panic!("Can't use each attribute on type that isn't a vec")
                        };
                        quote! {
                            pub fn #builder_attr_each(&mut self, it: #field_type) -> &mut Self {
                                self.#field_name.push(it);
                                self
                            }
                        }
                    } else {
                        default(&f.ty)
                    }
                }
                None => default(&f.ty),
            }
        })
        .collect();

    let builder_field_init: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let field_name = &f.ident.as_ref().unwrap();
            if is_vec(&f.ty) {
                quote! {
                    #field_name: vec![],
                }
            } else {
                quote! {
                    #field_name: None,
                }
            }
        })
        .collect();

    let result = quote! {
        pub struct #builder_name {
            #( #builder_field )*
        }

        impl #builder_name {
            #( #field_setter )*

            pub fn build(&mut self) -> std::result::Result<Command, std::boxed::Box<dyn std::error::Error>> {
                #( #field_conversion )*
                std::result::Result::Ok(Command {
                    #( #field_name : #field_name.clone(), )*
                })
            }
        }

        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #( #builder_field_init )*
                }
            }
        }
    };
    result.into()
}

fn get_inner_ty<'a>(t: &'a Type, expected_ident: &str) -> Option<&'a Type> {
    match t {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == expected_ident {
                    if let PathArguments::AngleBracketed(ref args) = segment.arguments {
                        if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                            return Some(inner_type);
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}
