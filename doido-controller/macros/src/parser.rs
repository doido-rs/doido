use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    Expr, Ident, LitStr, Result, Token,
};

pub enum RouteDecl {
    Method {
        method: String,
        path: LitStr,
        handler: Expr,
    },
    Resources {
        resource_name: Ident,
        controller: Ident,
        filter: ResourceFilter,
    },
    Namespace {
        name: Ident,
        body: RoutesInput,
    },
    Scope {
        path_prefix: LitStr,
        body: RoutesInput,
    },
}

pub enum ResourceFilter {
    All,
    Only(Vec<String>),
    Except(Vec<String>),
}

pub struct RoutesInput {
    pub decls: Vec<RouteDecl>,
}

fn parse_action_list(input: ParseStream) -> Result<Vec<String>> {
    let content;
    bracketed!(content in input);
    let mut actions = Vec::new();
    while !content.is_empty() {
        let ident: Ident = content.parse()?;
        actions.push(ident.to_string());
        let _comma: Option<Token![,]> = content.parse().ok();
    }
    Ok(actions)
}

impl Parse for RoutesInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut decls = Vec::new();
        while !input.is_empty() {
            let macro_ident: Ident = input.parse()?;
            let _bang: Token![!] = input.parse()?;
            let content;
            syn::parenthesized!(content in input);
            let _semi: Option<Token![;]> = input.parse().ok();

            match macro_ident.to_string().as_str() {
                "namespace" => {
                    let name: Ident = content.parse()?;
                    let _comma: Token![,] = content.parse()?;
                    let inner;
                    braced!(inner in content);
                    let body: RoutesInput = inner.parse()?;
                    decls.push(RouteDecl::Namespace { name, body });
                }
                "scope" => {
                    let path_prefix: LitStr = content.parse()?;
                    let _comma: Token![,] = content.parse()?;
                    let inner;
                    braced!(inner in content);
                    let body: RoutesInput = inner.parse()?;
                    decls.push(RouteDecl::Scope { path_prefix, body });
                }
                "resources" => {
                    let resource_name: Ident = content.parse()?;
                    let _comma: Token![,] = content.parse()?;
                    let controller: Ident = content.parse()?;
                    let filter = if content.is_empty() {
                        ResourceFilter::All
                    } else {
                        let _comma: Token![,] = content.parse()?;
                        let key: Ident = content.parse()?;
                        let _colon: Token![:] = content.parse()?;
                        let actions = parse_action_list(&content)?;
                        match key.to_string().as_str() {
                            "only" => ResourceFilter::Only(actions),
                            "except" => ResourceFilter::Except(actions),
                            other => {
                                return Err(syn::Error::new(
                                    key.span(),
                                    format!("unknown option: {other}"),
                                ))
                            }
                        }
                    };
                    decls.push(RouteDecl::Resources {
                        resource_name,
                        controller,
                        filter,
                    });
                }
                method @ ("get" | "post" | "put" | "patch" | "delete") => {
                    let path: LitStr = content.parse()?;
                    let _comma: Token![,] = content.parse()?;
                    let handler: Expr = content.parse()?;
                    decls.push(RouteDecl::Method {
                        method: method.to_string(),
                        path,
                        handler,
                    });
                }
                other => {
                    return Err(syn::Error::new(
                        macro_ident.span(),
                        format!("unknown macro: {other}!"),
                    ))
                }
            }
        }
        Ok(RoutesInput { decls })
    }
}
