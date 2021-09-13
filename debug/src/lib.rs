use darling::{
    ast,
    usage::{CollectTypeParams, GenericsExt, Purpose},
    uses_lifetimes, uses_type_params, FromDeriveInput, FromField,
};
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, parse_quote, DeriveInput, GenericParam, Generics};
#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let receiver = MyInputReceiver::from_derive_input(&input).unwrap();
    quote!(#receiver).into()
}

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_any))]
struct MyInputReceiver {
    /// The struct ident.
    ident: syn::Ident,

    /// The type's generics. You'll need these any time your trait is expected
    /// to work with types that declare generics.
    generics: syn::Generics,

    /// Receives the body of the struct or enum. We don't care about
    /// struct fields because we previously told darling we only accept structs.
    data: ast::Data<(), MyFieldReceiver>,
}
impl ToTokens for MyInputReceiver {
    fn to_tokens(&self, tokens: &mut quote::__private::TokenStream) {
        let MyInputReceiver {
            ref ident,
            ref generics,
            ref data,
        } = *self;
        let field_to_bound = get_fields_to_bound(data);
        let generics = add_trait_bounds(&generics, field_to_bound);
        let (imp, ty, wher) = generics.split_for_impl();
        let fields = data
            .as_ref()
            .take_struct()
            .expect("Should never be enum")
            .fields;
        let is_named = fields.first().map(|f| f.ident.is_some());

        match is_named {
            None => tokens.extend(quote! {
                #[allow(unused_must_use)]
                impl #imp std::fmt::Debug for #ident #ty #wher{
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error>{
                        write!(f,stringify!(#ident));
                        Ok(())
                    }
                }
            }),
            Some(is_named) => {
                let mut format_list = Vec::with_capacity(fields.len());
                let arg_list = fields
                    .into_iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let mut format = "{:?}".to_string();
                        for attr in f.attrs.iter() {
                            if let Ok(syn::Meta::NameValue(v)) = attr.parse_meta() {
                                if v.path.is_ident("debug") {
                                    match v.lit {
                                        syn::Lit::Str(s) => format = s.value(),
                                        _ => {
                                            let error = syn::Error::new_spanned(
                                                v.lit,
                                                "unsupported meta value",
                                            )
                                            .to_compile_error();
                                            tokens.extend(quote!(#error));
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                        format_list.push(format);

                        // This works with named or indexed fields, so we'll fall back to the index so we can
                        // write the output as a key-value pair.
                        let field_ident =
                            f.ident.as_ref().map(|v| quote!(#v)).unwrap_or_else(|| {
                                let i = syn::Index::from(i);
                                quote!(#i)
                            });

                        quote!(#field_ident)
                    })
                    .collect::<Vec<_>>();
                if is_named {
                    let format_str = arg_list
                        .iter()
                        .zip(format_list.iter())
                        .map(|(arg, format)| format!("{}: {}", arg, format))
                        .collect::<Vec<_>>()
                        .join(", ");
                    tokens.extend(quote! {
                        #[allow(unused_must_use)]
                        impl #imp std::fmt::Debug for #ident #ty #wher{
                            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error>{
                                write!(f,concat!(stringify!(#ident)," {{ ",#format_str," }}"),#(self.#arg_list),*);
                            Ok(())
                            }
                        }
                    });
                } else {
                    let format_str = format_list
                        .iter()
                        .map(|format| format!("{}", format))
                        .collect::<Vec<_>>()
                        .join(", ");
                    tokens.extend(quote! {
                        #[allow(unused_must_use)]
                        impl #imp std::fmt::Debug for #ident #ty #wher{
                            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error>{
                               write!(f,concat!(stringify!(#ident)," ( ",#format_str," )"),#(self.#arg_list),*);
                            Ok(())
                            }
                        }
                    });
                }
            }
        };
    }
}
#[derive(Debug, FromField)]
#[darling(forward_attrs(debug))]
struct MyFieldReceiver {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    attrs: Vec<syn::Attribute>,
}

fn add_trait_bounds(generics: &Generics, field_to_bound: Vec<&MyFieldReceiver>) -> Generics {
    let type_paras = generics.declared_type_params();
    let bound_set = field_to_bound.collect_type_params(&Purpose::BoundImpl.into(), &type_paras);
    let mut generics = generics.clone();
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            if bound_set.contains(&type_param.ident) {
                type_param.bounds.push(parse_quote!(std::fmt::Debug));
            }
        }
    }
    generics
}

uses_type_params!(MyFieldReceiver, ty);
uses_lifetimes!(MyFieldReceiver, ty);

fn get_fields_to_bound<'a>(
    data: &'a ast::Data<(), MyFieldReceiver>,
) -> Vec<&'a MyFieldReceiver> {
    let body = data.as_ref().take_struct().expect("Should never be enum");
    //dbg!(fields);
    body.fields
        .into_iter()
        .filter(|x| {
            if let syn::Type::Path(ref p) = x.ty {
                p.path
                    .segments
                    .last()
                    .map(|x| x.ident != "PhantomData")
                    .unwrap_or(true)
            } else {
                true
            }
        })
        .collect()
}
