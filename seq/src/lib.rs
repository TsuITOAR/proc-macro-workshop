use proc_macro::TokenStream;
use proc_macro2::{Group, Literal, TokenStream as TokenStream2, TokenTree};
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    Block, Ident, LitInt, Result, Token,
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
        let b = input.parse::<Block>()?;
        let body = if let Some(TokenTree::Group(g)) = b.to_token_stream().into_iter().next() {
            g.stream()
        } else {
            unreachable!()
        };
        Ok(Self {
            ident,
            lower_bound,
            higher_bound,
            body,
        })
    }
}

fn substitute_ident(input: TokenStream2, ident: Ident, target: TokenTree) -> TokenStream2 {
    let mut out = TokenStream2::new();
    for t in input {
        match t {
            TokenTree::Group(g) => TokenTree::Group(Group::new(
                g.delimiter(),
                substitute_ident(g.stream(), ident.clone(), target.clone()),
            )),
            TokenTree::Ident(i) => {
                if i == ident {
                    target.clone()
                } else {
                    TokenTree::Ident(i)
                }
            }
            x => x,
        }
        .to_tokens(&mut out)
    }
    out
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Seq);
    let s = (input.lower_bound..input.higher_bound).map(|i| {
        substitute_ident(
            input.body.clone(),
            input.ident.clone(),
            TokenTree::Literal(Literal::i32_unsuffixed(i)),
        )
    });
    quote_spanned! {input.body.span()=> #(#s)*}.into()
}
