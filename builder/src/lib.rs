use proc_macro::TokenStream;
use quote::{format_ident, quote};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    //eprintln!("INPUT:\n{:#?}", input);
    let struct_name = input.ident;
    let builder_name = format_ident!("{}Builder", struct_name);
    let output = quote! {
        impl #struct_name{
            fn builder()->#builder_name{
                #builder_name::default()
            }
        }
        #[derive(Default)]
        struct #builder_name{
            executable: Option<String>,
            args: Option<Vec<String>>,
            env: Option<Vec<String>>,
            current_dir: Option<String>,
        }
    };
    output.into()
}
