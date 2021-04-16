use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DataStruct};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    //eprintln!("INPUT:\n{:#?}", input);
    let struct_name = input.ident;
    let builder_name = format_ident!("{}Builder", struct_name);
    let struct_fields = match input.data {
        Data::Struct(DataStruct { fields, .. }) => fields,
        _ => unimplemented!(),
    };
    let builder_fields = struct_fields
        .iter()
        .map(|field| (field.ident.as_ref().unwrap(), &field.ty))
        .map(|(ident, ty)| quote!(#ident: Option<#ty>));
    let builder_methods = struct_fields
        .iter()
        .map(|field| (field.ident.as_ref().unwrap(), &field.ty))
        .map(|(ident, ty)| {
            quote! {fn #ident(&mut self,value:#ty)->&mut Self{
                    self.#ident=Some(value);
                    self
                }
            }
        });

    let output = quote! {
        impl #struct_name{
            fn builder()->#builder_name{
                #builder_name::default()
            }
        }
        #[derive(Default)]
        struct #builder_name{
            #(#builder_fields),*
        }

        impl #builder_name{
            #(#builder_methods)*
        }
    };
    output.into()
}
