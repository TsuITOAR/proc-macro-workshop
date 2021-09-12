use darling::{ast, FromDeriveInput, FromField, FromMeta};
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
#[proc_macro_derive(CustomDebug)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = syn::parse(input).unwrap();
    let receiver = MyInputReceiver::from_derive_input(&input).unwrap();
    quote!(#receiver).into()
}

#[derive(Debug, Clone, FromMeta)]
struct Format {
    format: String,
}
impl Default for Format {
    fn default() -> Self {
        Self {
            format: "{:?}".to_string(),
        }
    }
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
                let format_list = fields
                    .into_iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let format = f.format.as_ref().map(|x| x.format.clone()).unwrap_or("{:?}".to_string());
                        // This works with named or indexed fields, so we'll fall back to the index so we can
                        // write the output as a key-value pair.
                        let field_ident =
                            f.ident.as_ref().map(|v| quote!(#v)).unwrap_or_else(|| {
                                let i = syn::Index::from(i);
                                quote!("{:?}, ",self.#i)
                            });
                        quote!(concat!(stringify!(#field_ident),": ",#format,", "),self.#field_ident)
                    })
                    .collect::<Vec<_>>();
                if is_named {
                    tokens.extend(quote! {
                        #[allow(unused_must_use)]
                        impl #imp std::fmt::Debug for #ident #ty #wher{
                            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error>{
                                write!(f,concat!(stringify!(#ident)," {{ "));
                                #(write!(f,#format_list);)*
                                write!(f,"}};");
                            Ok(())
                            }
                        }
                    });
                } else {
                    tokens.extend(quote! {
                        #[allow(unused_must_use)]
                        impl #imp std::fmt::Debug for #ident #ty #wher{
                            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error>{
                                write!(f,concat!(stringify!(#ident)," ( "));
                                #(write!(f,#format_list);)*
                                write!(f,");");
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
#[darling(attributes(debug))]
struct MyFieldReceiver {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    #[darling(default)]
    format: Option<Format>,
}
