#![feature(proc_macro_diagnostic)]
use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, Literal, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote, ToTokens};
use syn::{
    braced, parenthesized,
    parse::{Parse, ParseStream},
    parse2, parse_macro_input, Ident, LitInt, Result, Token,
};

#[derive(Debug, Clone)]
struct Seq {
    ident: Ident,
    lower_bound: i32,
    higher_bound: i32,
    body: RepeatBody,
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

impl ToTokens for Seq {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let iter = (self.lower_bound..self.higher_bound)
            .map(|x| TokenTree::Literal(Literal::i32_unsuffixed(x)));
        match self.body {
            RepeatBody::RepAll(ref a) => {
                tokens.extend(iter.map(|x| substitute_ident(a.clone(), &self.ident, &x)));
            }
            RepeatBody::RepPart(ref r) => r.clone().to_tokens(&self.ident, iter, tokens),
        }
    }
}

#[derive(Debug, Clone)]
enum RepeatBody {
    RepAll(TokenStream2),
    RepPart(RepGroup),
}
#[derive(Debug, Clone)]
enum RepSec {
    Group { delim: Delimiter, content: RepGroup },
    RepNode(TokenStream2),
    NonRep(TokenStream2),
}

#[derive(Debug, Clone)]
struct RepGroup {
    s: Vec<RepSec>,
}

impl RepGroup {
    fn to_tokens<Iter: Iterator<Item = TokenTree> + Clone>(
        self,
        src: &Ident,
        dst: Iter,
        s: &mut TokenStream2,
    ) {
        for i in self.s {
            match i {
                RepSec::Group { delim, content } => {
                    let mut ts = TokenStream2::new();
                    content.to_tokens(&src, dst.clone(), &mut ts);
                    Group::new(delim, ts).to_tokens(s)
                }
                RepSec::NonRep(t) => t.to_tokens(s),
                RepSec::RepNode(r) => s.extend(
                    dst.clone()
                        .into_iter()
                        .map(|i| substitute_ident(r.clone(), src, &i)),
                ),
            }

            dbg!(&s);
        }
    }
}

impl Parse for RepGroup {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut s = Vec::new();
        let rep_sec;
        while !input.is_empty() {
            dbg!(input);
            if input.peek(Token![#]) && input.peek2(syn::token::Paren) {
                input.parse::<Token![#]>()?;
                parenthesized!(rep_sec in input);
                s.push(RepSec::RepNode(rep_sec.parse()?));
                input.parse::<Token![*]>()?;
                s.push(RepSec::NonRep(input.parse()?));
                break;
            } else if let Ok(g) = input.parse::<Group>() {
                s.push(RepSec::Group {
                    delim: g.delimiter(),
                    content: parse2(g.stream())?,
                })
            } else {
                s.push(RepSec::NonRep(
                    input.parse::<TokenTree>()?.to_token_stream(),
                ));
            }
        }
        Ok(dbg!(RepGroup { s }))
    }
}

impl Parse for RepeatBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut pre = TokenStream2::new();
        let fork = input.fork();
        loop {
            dbg!(input);
            if input.peek(Token![#]) && input.peek2(syn::token::Paren) {
                break;
            } else if input.is_empty() {
                return Ok(RepeatBody::RepAll(pre));
            } else {
                if let Ok(g) = input.parse::<Group>() {
                    match parse2::<RepeatBody>(g.stream())? {
                        RepeatBody::RepAll(_) => g.to_tokens(&mut pre),
                        RepeatBody::RepPart(p) => {
                            return Ok(RepeatBody::RepPart(RepGroup {
                                s: vec![
                                    RepSec::NonRep(pre),
                                    RepSec::Group {
                                        delim: g.delimiter(),
                                        content: p,
                                    },
                                    RepSec::NonRep(dbg!(input.parse())?),
                                ],
                            }))
                        }
                    }
                } else {
                    input.parse::<TokenTree>()?.to_tokens(&mut pre);
                }
            }
        }
        Ok(RepeatBody::RepPart(fork.parse()?))
    }
}

fn substitute_ident(input: TokenStream2, ident: &Ident, target: &TokenTree) -> TokenStream2 {
    let mut out = TokenStream2::new();
    let mut concat_ident = None;
    let mut last_ident = None;
    for t in input {
        match t {
            TokenTree::Group(g) => {
                last_ident.take().to_tokens(&mut out);
                let mut ng = TokenTree::Group(Group::new(
                    g.delimiter(),
                    substitute_ident(g.stream(), ident, target),
                ));
                ng.set_span(g.span());
                ng.to_tokens(&mut out)
            }
            TokenTree::Ident(i) => {
                if i == *ident {
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

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Seq);
    dbg!(&input);
    quote! {#input}.into()
}
