use darling::{ast, FromDeriveInput, FromField};
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = syn::parse(input).expect("parse token stream failed");
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
                        write!(f,concat!(stringify!(#ident),";"));
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
                                            tokens.extend(quote!(#error))
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
