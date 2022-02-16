#![feature(proc_macro_diagnostic)]
use proc_macro::TokenStream;
use proc_macro2::{Group, Literal, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, LitInt, Result, Token,
};

#[derive(Debug, Clone)]
struct Seq {
    ident: Ident,
    lower_bound: i32,
    higher_bound: i32,
    body: TokenStream2,
}

impl Parse for Seq {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident: Ident = input.parse()?;
        input.parse::<Token![in]>()?;
        let lower_bound = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Token![..]>()?;
        let higher_bound = input.parse::<LitInt>()?.base10_parse()?;
        let body;
        braced!(body in input);
        Ok(Self {
            ident,
            lower_bound,
            higher_bound,
            body: body.parse()?,
        })
    }
}

fn substitute_ident(input: TokenStream2, ident: Ident, target: &TokenTree) -> TokenStream2 {
    let mut out = TokenStream2::new();
    let mut concat_ident = None;
    let mut last_ident = None;
    for t in input {
        match t {
            TokenTree::Group(g) => {
                last_ident.take().to_tokens(&mut out);
                let mut ng = TokenTree::Group(Group::new(
                    g.delimiter(),
                    substitute_ident(g.stream(), ident.clone(), target),
                ));
                ng.set_span(g.span());
                ng.to_tokens(&mut out)
            }
            TokenTree::Ident(i) => {
                if i == ident {
                    match concat_ident.take() {
                        Some(x) => format_ident!("{}{}", x, target.to_string()).to_tokens(&mut out),
                        None => target.to_tokens(&mut out),
                    }
                } else {
                    match concat_ident.take() {
                        Some(_) => i.span().unwrap().error("mismatched ident after '#'").emit(),
                        None => last_ident.replace(i).to_tokens(&mut out),
                    }
                }
            }
            TokenTree::Punct(p) if p.as_char() == '#' => match last_ident.take() {
                Some(l) => concat_ident = Some(l),
                None => p
                    .span()
                    .unwrap()
                    .error("no ident before '#' to concat")
                    .emit(),
            },
            x => {
                last_ident.take().to_tokens(&mut out);
                x.to_tokens(&mut out)
            }
        }
    }
    last_ident.to_tokens(&mut out);
    if let Some(i) = concat_ident {
        i.span()
            .unwrap()
            .error("no ident after '#' to concat")
            .emit()
    }
    out
}
enum T{
    One(u8),
    Two
}
#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Seq);
    let s = (input.lower_bound..input.higher_bound).map(|i| {
        substitute_ident(
            input.body.clone(),
            input.ident.clone(),
            &TokenTree::Literal(Literal::i32_unsuffixed(i)),
        )
    });
    quote! {#(#s)*}.into()
}
