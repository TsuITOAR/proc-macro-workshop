use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{spanned::Spanned, Data, DataStruct};
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
    let is_option = |ty: &syn::Type| match ty {
        syn::Type::Path(
            syn::TypePath {
                path: syn::Path { segments: s, .. },
                ..
            },
            ..,
        ) if s.len() == 1 && s[0].ident == "Option" => Some(s[0].arguments.clone()),
        _ => None,
    };
    let builder_fields = struct_fields
        .iter()
        .map(|field| (field.ident.as_ref().unwrap(), &field.ty))
        .map(|(ident, ty)| {
            if is_option(ty).is_some() {
                quote! {#ident:#ty}
            } else {
                quote! {#ident: Option<#ty>}
            }
        });
    let builder_methods = struct_fields
        .iter()
        .map(|field| (field.ident.as_ref().unwrap(), &field.ty))
        .map(|(ident, ty)| {
            match is_option(ty) {
                Some(syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    args,
                    ..
                })) if args.len() == 1 => {
                    let unwrap_type = args.first().unwrap();
                    quote! {fn #ident(&mut self,value:#unwrap_type)->&mut Self{
                        self.#ident=Some(value);
                        self
                    }}
                }
                Some(_) => {
                    quote_spanned!(ty.span()=>compile_error!("Option need to wrap a single type"))
                }
                None => quote! {
                    fn #ident(&mut self,value:#ty)->&mut Self{
                        self.#ident=Some(value);
                        self
                    }
                },
            }
        });
    let members = struct_fields
        .iter()
        .map(|field| (field.ident.as_ref().unwrap()));
    let set_members = struct_fields
        .iter()
        .map(|field| (field.ident.as_ref().unwrap(), &field.ty))
        .map(|(ident, ty)| {
			if is_option(ty).is_some() {
                quote! {let #ident=&self.#ident}
            } else {
                quote! {let #ident=self.#ident.as_ref().ok_or(concat!("Field ",stringify!(#ident)," not set",))?}
            }
		});
    let output = quote! {
        impl #struct_name{
            fn builder()->#builder_name{
                #builder_name::default()
            }
        }
		#[allow(dead_code)]
        #[derive(Default)]
        struct #builder_name{
            #(#builder_fields),*
        }

        impl #builder_name{
            #(#builder_methods)*
        }
        impl #builder_name{
            fn build(&self)->Result<#struct_name, Box<dyn std::error::Error>>{
                #(#set_members;)*
                Ok(#struct_name{
                    #(#members:#members.clone()),*
                })
            }
        }
    };
    output.into()
}
