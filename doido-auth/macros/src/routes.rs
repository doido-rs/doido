use doido_auth_route_dsl::expand_auth_route_decls;
use proc_macro2::{TokenStream, TokenTree};
use quote::quote;
use syn::{parse::ParseStream, parse2, Result};

struct RoutesInput {
    body: TokenStream,
}

impl syn::parse::Parse for RoutesInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut body = TokenStream::new();
        while !input.is_empty() {
            body.extend(std::iter::once(input.parse::<TokenTree>()?));
        }
        Ok(Self { body })
    }
}

fn expand_body(body: TokenStream, api_only: bool) -> TokenStream {
    let tokens: Vec<TokenTree> = body.into_iter().collect();
    let mut out = TokenStream::new();
    let mut i = 0;

    while i < tokens.len() {
        if let TokenTree::Ident(ident) = &tokens[i] {
            if ident == "auth_routes"
                && i + 2 < tokens.len()
                && matches!(&tokens[i + 1], TokenTree::Punct(p) if p.as_char() == '!')
            {
                if let TokenTree::Group(group) = &tokens[i + 2] {
                    if group.delimiter() == proc_macro2::Delimiter::Parenthesis {
                        out.extend(expand_auth_route_decls(group.stream(), api_only));
                        i += 3;
                        if i < tokens.len() {
                            if let TokenTree::Punct(p) = &tokens[i] {
                                if p.as_char() == ';' {
                                    i += 1;
                                }
                            }
                        }
                        continue;
                    }
                }
            }
        }

        out.extend(std::iter::once(tokens[i].clone()));
        i += 1;
    }

    out
}

pub fn expand_routes(input: TokenStream, api_only: bool) -> TokenStream {
    let parsed: RoutesInput = parse2(input).unwrap_or(RoutesInput {
        body: TokenStream::new(),
    });
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
