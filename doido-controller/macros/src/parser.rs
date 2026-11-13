use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    Expr, Ident, LitBool, LitStr, Result, Token,
};

/// Member vs collection route scoped to the enclosing `resource!` block.
#[derive(Clone, Copy, Debug)]
pub enum ResourceScope {
    Member,
    Collection,
}

pub enum RouteDecl {
    Method {
        method: String,
        /// Absent when `resource_scope` is set (path derived from enclosing resource).
        path: Option<LitStr>,
        handler: Expr,
        name: Option<Ident>,
        constraints: Vec<(Ident, Expr)>,
        resource_scope: Option<(ResourceScope, String)>,
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
    Resource {
        name: Ident,
        controller: Ident,
        singular: bool,
        filter: ResourceFilter,
        body: Option<RoutesInput>,
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

fn parse_method_decl(macro_ident: &Ident, content: ParseStream) -> Result<RouteDecl> {
    let method = macro_ident.to_string();

    // `get!(member: "preview", Handler)` / `get!(collection: "search", Handler)`
    if content.peek(Ident) {
        let scope_kw: Ident = content.parse()?;
        match scope_kw.to_string().as_str() {
            "member" | "collection" => {
                let _colon: Token![:] = content.parse()?;
                let segment: LitStr = content.parse()?;
                let _comma: Token![,] = content.parse()?;
                let handler: Expr = content.parse()?;
                let scope = if scope_kw == "member" {
                    ResourceScope::Member
                } else {
                    ResourceScope::Collection
                };
                return Ok(RouteDecl::Method {
                    method,
                    path: None,
                    handler,
                    name: None,
                    constraints: Vec::new(),
                    resource_scope: Some((scope, segment.value())),
                });
            }
            _ => {}
        }
    }

    let path: LitStr = content.parse()?;
    let _comma: Token![,] = content.parse()?;
    let handler: Expr = content.parse()?;
    let mut name = None;
    let mut constraints = Vec::new();
    while content.peek(Token![,]) {
        let _comma: Token![,] = content.parse()?;
        if content.peek(Token![as]) {
            let _as: Token![as] = content.parse()?;
            let _colon: Token![:] = content.parse()?;
            name = Some(content.parse()?);
        } else {
            let key: Ident = content.parse()?;
            let _colon: Token![:] = content.parse()?;
            match key.to_string().as_str() {
                "constraints" => {
                    let inner;
                    braced!(inner in content);
                    while !inner.is_empty() {
                        let param: Ident = inner.parse()?;
                        let _colon: Token![:] = inner.parse()?;
                        let validator: Expr = inner.parse()?;
                        constraints.push((param, validator));
                        let _comma: Option<Token![,]> = inner.parse().ok();
                    }
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown route option: {other}"),
                    ))
                }
            }
        }
    }
    Ok(RouteDecl::Method {
        method,
        path: Some(path),
        handler,
        name,
        constraints,
        resource_scope: None,
    })
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
                    let mut singular = false;
                    let mut filter = ResourceFilter::All;
                    let mut body = None;
                    while !content.is_empty() {
                        let _comma: Token![,] = content.parse()?;
                        if content.peek(syn::token::Brace) {
                            let inner;
                            braced!(inner in content);
                            body = Some(inner.parse()?);
                            break;
                        }
                        let key: Ident = content.parse()?;
                        let _colon: Token![:] = content.parse()?;
                        match key.to_string().as_str() {
                            "only" => filter = ResourceFilter::Only(parse_action_list(&content)?),
                            "except" => {
                                filter = ResourceFilter::Except(parse_action_list(&content)?)
                            }
                            "singular" => {
                                singular = content.parse::<LitBool>()?.value;
                            }
                            other => {
                                return Err(syn::Error::new(
                                    key.span(),
                                    format!("unknown resource option: {other}"),
                                ))
                            }
                        }
                    }
                    decls.push(RouteDecl::Resource {
                        name,
                        controller,
                        singular,
                        filter,
                        body,
                    });
                }
                "resources" => {
                    return Err(syn::Error::new(
                        macro_ident.span(),
                        "`resources!` was removed; use `resource!` instead (plural REST is the default)",
                    ));
                }
                "shallow_resources" => {
                    return Err(syn::Error::new(
                        macro_ident.span(),
                        "`shallow_resources!` was removed; nest with `resource!(parent, Ctrl, { resource!(child, Ctrl) })`",
                    ));
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
                method @ ("get" | "post" | "put" | "patch" | "delete" | "head" | "options") => {
                    decls.push(parse_method_decl(
                        &Ident::new(method, macro_ident.span()),
                        &content,
                    )?);
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
