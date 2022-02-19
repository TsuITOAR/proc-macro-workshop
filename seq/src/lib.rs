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
    body: Body,
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
            Body::RepAll(ref a) => {
                tokens.extend(iter.map(|x| substitute_ident(a.clone(), &self.ident, &x)));
            }
            Body::RepPart(ref r) => r.clone().to_tokens(&self.ident, iter, tokens),
        }
    }
}

#[derive(Debug, Clone)]
enum Body {
    RepAll(TokenStream2),
    RepPart(RepStream),
}
#[derive(Debug, Clone)]
enum RepToken {
    Group {
        delim: Delimiter,
        content: RepStream,
    },
    RepNode(TokenStream2),
    NonRep(TokenStream2),
}

#[derive(Debug, Clone)]
struct RepStream {
    s: Vec<RepToken>,
}

impl RepStream {
    fn to_tokens<Iter: Iterator<Item = TokenTree> + Clone>(
        self,
        src: &Ident,
        dst: Iter,
        s: &mut TokenStream2,
    ) {
        for i in self.s {
            match i {
                RepToken::Group { delim, content } => {
                    let mut ts = TokenStream2::new();
                    content.to_tokens(&src, dst.clone(), &mut ts);
                    Group::new(delim, ts).to_tokens(s)
                }
                RepToken::NonRep(t) => t.to_tokens(s),
                RepToken::RepNode(r) => s.extend(
                    dst.clone()
                        .into_iter()
                        .map(|i| substitute_ident(r.clone(), src, &i)),
                ),
            }
        }
    }
}

impl Parse for RepStream {
    fn parse(input: ParseStream) -> Result<Self> {
        //let _s = DebugGuard::new(format!("parsing RepeatSec: {}", input.to_string()));
        let mut s = Vec::new();
        let rep_sec;
        while !input.is_empty() {
            if input.peek(Token![#]) && input.peek2(syn::token::Paren) {
                input.parse::<Token![#]>()?;
                parenthesized!(rep_sec in input);
                s.push(RepToken::RepNode(rep_sec.parse()?));
                input.parse::<Token![*]>()?;
                s.push(RepToken::NonRep(input.parse()?));
                break;
            } else if let Ok(g) = input.parse::<Group>() {
                s.push(RepToken::Group {
                    delim: g.delimiter(),
                    content: parse2(g.stream())?,
                });
            } else {
                s.push(RepToken::NonRep(
                    input.parse::<TokenTree>()?.to_token_stream(),
                ));
            }
        }
        Ok(RepStream { s })
    }
}

impl Parse for Body {
    fn parse(input: ParseStream) -> Result<Self> {
        //let _s = DebugGuard::new(format!("parsing Body: {}", input.to_string()));
        let mut pre = TokenStream2::new();
        let fork = input.fork();
        loop {
            if fork.peek(Token![#]) && fork.peek2(syn::token::Paren) {
                break;
            } else if fork.is_empty() {
                return Ok(Body::RepAll(input.parse()?));
            } else {
                if let Ok(g) = fork.parse::<Group>() {
                    match parse2::<Self>(g.stream())? {
                        Body::RepAll(_) => g.to_tokens(&mut pre),
                        Body::RepPart(_) => {
                            return Ok(Body::RepPart(input.parse()?));
                        }
                    }
                } else {
                    fork.parse::<TokenTree>()?.to_tokens(&mut pre);
                }
            }
        }

        Ok(Body::RepPart(input.parse()?))
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
    quote! {#input}.into()
}
/* 
struct DebugGuard {
    info: String,
}

impl DebugGuard {
    fn new(info: String) -> Self {
        eprintln!("starting {}", info);
        Self { info }
    }
}

impl Drop for DebugGuard {
    fn drop(&mut self) {
        eprintln!("ending {}", &self.info);
    }
}
 */