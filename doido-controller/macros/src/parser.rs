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
        /// Optional route name for a generated `{name}_path()` helper (`as: name`).
        name: Option<Ident>,
    },
    Root {
        handler: Expr,
    },
    Redirect {
        from: LitStr,
        to: LitStr,
    },
    Mount {
        path: LitStr,
        router: Expr,
    },
    Resources {
        resource_name: Ident,
        controller: Ident,
        filter: ResourceFilter,
        /// Extra GET routes under `/{id}/<action>` (Rails `member do … end`).
        member: Vec<String>,
        /// Extra GET routes under `/<action>` (Rails `collection do … end`).
        collection: Vec<String>,
    },
    Resource {
        name: Ident,
        controller: Ident,
    },
    ShallowResources {
        parent: Ident,
        child: Ident,
        controller: Ident,
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
                "resource" => {
                    let name: Ident = content.parse()?;
                    let _comma: Token![,] = content.parse()?;
                    let controller: Ident = content.parse()?;
                    decls.push(RouteDecl::Resource { name, controller });
                }
                "shallow_resources" => {
                    let parent: Ident = content.parse()?;
                    let _comma: Token![,] = content.parse()?;
                    let child: Ident = content.parse()?;
                    let _comma: Token![,] = content.parse()?;
                    let controller: Ident = content.parse()?;
                    decls.push(RouteDecl::ShallowResources {
                        parent,
                        child,
                        controller,
                    });
                }
                "resources" => {
                    let resource_name: Ident = content.parse()?;
                    let _comma: Token![,] = content.parse()?;
                    let controller: Ident = content.parse()?;
                    // Zero or more `, key: [actions]` options (only/except/member/collection).
                    let mut filter = ResourceFilter::All;
                    let mut member = Vec::new();
                    let mut collection = Vec::new();
                    while !content.is_empty() {
                        let _comma: Token![,] = content.parse()?;
                        let key: Ident = content.parse()?;
                        let _colon: Token![:] = content.parse()?;
                        let actions = parse_action_list(&content)?;
                        match key.to_string().as_str() {
                            "only" => filter = ResourceFilter::Only(actions),
                            "except" => filter = ResourceFilter::Except(actions),
                            "member" => member = actions,
                            "collection" => collection = actions,
                            other => {
                                return Err(syn::Error::new(
                                    key.span(),
                                    format!("unknown option: {other}"),
                                ))
                            }
                        }
                    }
                    decls.push(RouteDecl::Resources {
                        resource_name,
                        controller,
                        filter,
                        member,
                        collection,
                    });
                }
                "root" => {
                    let handler: Expr = content.parse()?;
                    decls.push(RouteDecl::Root { handler });
                }
                "redirect" => {
                    let from: LitStr = content.parse()?;
                    let _comma: Token![,] = content.parse()?;
                    let to: LitStr = content.parse()?;
                    decls.push(RouteDecl::Redirect { from, to });
                }
                "mount" => {
                    let path: LitStr = content.parse()?;
                    let _comma: Token![,] = content.parse()?;
                    let router: Expr = content.parse()?;
                    decls.push(RouteDecl::Mount { path, router });
                }
                method @ ("get" | "post" | "put" | "patch" | "delete") => {
                    let path: LitStr = content.parse()?;
                    let _comma: Token![,] = content.parse()?;
                    let handler: Expr = content.parse()?;
                    // Optional `, as: name` for a named `{name}_path()` helper.
                    // `as` is a keyword, so match it as a token, not an ident.
                    let mut name = None;
                    if content.peek(Token![,]) {
                        let _comma: Token![,] = content.parse()?;
                        let _as: Token![as] = content.parse()?;
                        let _colon: Token![:] = content.parse()?;
                        name = Some(content.parse()?);
                    }
                    decls.push(RouteDecl::Method {
                        method: method.to_string(),
                        path,
                        handler,
                        name,
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
