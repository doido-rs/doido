use doido_auth_route_dsl::expand_auth_route_decls;
use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::quote;
use syn::{parse::ParseStream, parse_macro_input, Token};

struct RoutesInput {
    body: TokenStream,
}

impl syn::parse::Parse for RoutesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            body: input.parse().unwrap_or_default(),
        })
    }
}

fn take_auth_routes_body(iter: &mut std::vec::IntoIter<TokenTree>) -> Option<TokenStream> {
    let first = iter.next()?;
    let TokenTree::Ident(ident) = first else {
        return None;
    };
    if ident != "auth_routes" {
        return None;
    }

    let bang = iter.next()?;
    let TokenTree::Punct(p) = bang else {
        return None;
    };
    if p.as_char() != '!' {
        return None;
    }

    let group = iter.next()?;
    let TokenTree::Group(g) = group else {
        return None;
    };
    if g.delimiter() != proc_macro2::Delimiter::Parenthesis {
        return None;
    }

    Some(g.stream())
}

fn expand_body(body: TokenStream, api_only: bool) -> TokenStream {
    let mut out = TokenStream::new();
    let mut iter = body.into_iter().peekable();

    while let Some(token) = &iter.peek() {
        if let TokenTree::Ident(ident) = token {
            if ident == "auth_routes" {
                let mut scan = iter.clone();
                scan.next();
                if let Some(body_tokens) = take_auth_routes_body(&mut scan) {
                    iter = scan;
                    out.extend(expand_auth_route_decls(body_tokens, api_only));
                    if iter.peek().is_some_and(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == ';')) {
                        iter.next();
                    }
                    continue;
                }
            }
        }

        out.extend(std::iter::once(iter.next().unwrap()));
    }

    out
}

pub fn expand_routes(input: TokenStream, api_only: bool) -> TokenStream {
    let parsed = parse_macro_input!(input as RoutesInput);
    let body = expand_body(parsed.body, api_only);
    quote! {
        doido_controller::routes! {
            #body
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn replaces_auth_routes_with_route_decls() {
        let input = quote! {
            auth_routes!(User);
            get!("/", HomeController::index);
        };
        let expanded = expand_body(input, false);
        let s = expanded.to_string();
        assert!(s.contains("post !"));
        assert!(s.contains("sign_in"));
        assert!(s.contains("HomeController :: index"));
    }
}
