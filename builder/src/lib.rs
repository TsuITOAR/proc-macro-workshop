use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{spanned::Spanned, Data, DataStruct};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    //eprintln!("INPUT:\n{:#?}", input);
    let struct_name = input.ident;
    let builder_name = format_ident!("{}Builder", struct_name);
    let struct_fields = match input.data {
        Data::Struct(DataStruct { fields, .. }) => fields,
        _ => unimplemented!("Syntax error"),
    };
    let is_wrapped = |ty: &syn::Type,wrapper:&str| match ty {
        syn::Type::Path(
            syn::TypePath {
                path: syn::Path { segments: s, .. },
                ..
            },
            ..,
        ) if s.len() == 1 && s[0].ident == wrapper => Some(s[0].arguments.clone()),
        _ => None,
    };
	let is_option = |ty: &syn::Type| is_wrapped(ty,"Option");
	let is_vec=|ty: &syn::Type| is_wrapped(ty,"Vec");

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
        .map(|field| (field.ident.as_ref().unwrap(), &field.ty, &field.attrs))
        .map(|(ident, ty, attrs)| match is_option(ty) {
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
            None => {
                let mut push_method_name: Option<Result<_, _>> = None;
                for attr in attrs {
                    if attr.path.is_ident("builder") {
                        let attr_body = attr.parse_meta().unwrap();
                        //eprintln!("{:#?}", attr_body);
                        if let syn::Meta::List(syn::MetaList { nested, .. }) = attr_body {
                            for meta in &nested {
                                match meta {
                                    syn::NestedMeta::Meta(syn::Meta::NameValue(
                                        syn::MetaNameValue {
                                            path,
                                            lit: syn::Lit::Str(lit_str),
                                            ..
                                        },
                                    )) => {
                                        if path.is_ident("each") {
											match lit_str.parse::<syn::Ident>(){
												Ok(ident)=>{
													push_method_name=Some(Ok(ident));
												},
												Err(_)=>{
													push_method_name=Some(Err(quote_spanned!{lit_str.span()=>compile_error!("Expect str")}));
												}
											}
                                        } else {
											push_method_name=Some(Err(quote_spanned!{meta.span()=>compile_error!("expected `builder(each = \"...\")`")}));
                                        }
                                    }
                                    _ => {
										unimplemented!("unsupported attribute type");
									}
                                }
                            }
                        } else {
                            //eprintln!("{:#?}", attr_body);
                            unimplemented!("meta parse failed");
                        }
                    }
                }
                match push_method_name {
                    None => quote! {
                        fn #ident(&mut self,value:#ty)->&mut Self{
                            self.#ident=Some(value);
                            self
                        }
                    },
                    Some(Ok(ref push_method_name)) => {
						match is_vec(ty) {
							Some(syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
								args,
								..
							})) if args.len() == 1 => {
								let unwrap_type = args.first().unwrap();
								if push_method_name != ident {
									quote! {
										fn #push_method_name(&mut self,value:#unwrap_type)->&mut Self{
											match self.#ident{
												Some(ref mut v)=>{v.push(value);},
												None=>{self.#ident=Some(vec![value]);}
											}
											self
										}
										fn #ident(&mut self,value:#ty)->&mut Self{
											self.#ident=Some(value);
											self
										}
									}
								} else {
									quote! {
										fn #push_method_name(&mut self,value:#unwrap_type)->&mut Self{
											match self.#ident{
												Some(ref mut v)=>{v.push(value);},
												None=>{self.#ident=Some(vec![value]);}
											}
											self
										}
									}
								}
							}
							_ => {
								quote_spanned!(ty.span()=>compile_error!("Option need to wrap a single type"))
							}
						}
                        
                    }
                    Some(Err(err_message)) => err_message,
                }
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
                quote! {let #ident=self.#ident.clone()}
            } else if is_vec(ty).is_some(){
				quote!{
					let #ident=match self.#ident{
						None=>Vec::new(),
						Some(ref v)=>v.clone()
					}
				}
			}else {
                quote! {let #ident=self.#ident.as_ref().ok_or(concat!("Field ",stringify!(#ident)," not set",))?.clone()}
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
                    #(#members:#members),*
                })
            }
        }
    };
    output.into()
}
