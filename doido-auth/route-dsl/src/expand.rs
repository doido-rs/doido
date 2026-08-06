use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{braced, bracketed, Expr, Ident, LitStr, Result, Token};

struct AuthRoutesInput {
    user: Ident,
    only: Option<Vec<String>>,
    skip: Option<Vec<String>>,
    prefix: Option<String>,
    controllers: std::collections::HashMap<String, Expr>,
    actions: std::collections::HashMap<String, std::collections::HashMap<String, Expr>>,
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

fn parse_controller_map(input: ParseStream) -> Result<std::collections::HashMap<String, Expr>> {
    let inner;
    braced!(inner in input);
    let mut map = std::collections::HashMap::new();
    while !inner.is_empty() {
        let key: Ident = inner.parse()?;
        let _colon: Token![:] = inner.parse()?;
        let value: Expr = inner.parse()?;
        map.insert(key.to_string(), value);
        let _comma: Option<Token![,]> = inner.parse().ok();
    }
    Ok(map)
}

fn parse_actions_map(
    input: ParseStream,
) -> Result<std::collections::HashMap<String, std::collections::HashMap<String, Expr>>> {
    let inner;
    braced!(inner in input);
    let mut map = std::collections::HashMap::new();
    while !inner.is_empty() {
        let module: Ident = inner.parse()?;
        let _colon: Token![:] = inner.parse()?;
        let actions_inner;
        braced!(actions_inner in inner);
        let mut actions = std::collections::HashMap::new();
        while !actions_inner.is_empty() {
            let action: Ident = actions_inner.parse()?;
            let _colon: Token![:] = actions_inner.parse()?;
            let handler: Expr = actions_inner.parse()?;
            actions.insert(action.to_string(), handler);
            let _comma: Option<Token![,]> = actions_inner.parse().ok();
        }
        map.insert(module.to_string(), actions);
        let _comma: Option<Token![,]> = inner.parse().ok();
    }
    Ok(map)
}

impl Parse for AuthRoutesInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let user: Ident = input.parse()?;
        let mut only = None;
        let mut skip = None;
        let mut prefix = None;
        let mut controllers = std::collections::HashMap::new();
        let mut actions = std::collections::HashMap::new();

        while input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
            let key: Ident = input.parse()?;
            let _colon: Token![:] = input.parse()?;
            match key.to_string().as_str() {
                "only" => only = Some(parse_action_list(input)?),
                "skip" => skip = Some(parse_action_list(input)?),
                "prefix" => {
                    let lit: LitStr = input.parse()?;
                    prefix = Some(lit.value());
                }
                "controllers" => controllers = parse_ident_map(input)?,
                "actions" => actions = parse_actions_map(input)?,
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown auth_routes option: {other}"),
                    ));
                }
            }
        }

        Ok(Self {
            user,
            only,
            skip,
            prefix,
            controllers,
            actions,
        })
    }
}

fn module_enabled(name: &str, only: &Option<Vec<String>>, skip: &Option<Vec<String>>) -> bool {
    if let Some(list) = only {
        return list.iter().any(|m| m == name);
    }
    if let Some(list) = skip {
        return !list.iter().any(|m| m == name);
    }
    true
}

fn join_path(prefix: &str, suffix: &str) -> String {
    let prefix = prefix.trim_end_matches('/');
    let suffix = suffix.trim_start_matches('/');
    if suffix.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{suffix}")
    }
}

fn handler_for(
    module: &str,
    action: &str,
    user: &Ident,
    controllers: &std::collections::HashMap<String, Ident>,
    actions: &std::collections::HashMap<String, std::collections::HashMap<String, Expr>>,
) -> TokenStream {
    if let Some(module_actions) = actions.get(module) {
        if let Some(expr) = module_actions.get(action) {
            return quote! { #expr };
        }
    }

    let action_ident = format_ident!("{}", action);

    if let Some(controller) = controllers.get(module) {
        return quote! { #controller::#action_ident };
    }

    match module {
        "sessions" => quote! { doido_auth::controllers::AuthSessions::<#user>::#action_ident },
        "registrations" => {
            quote! { doido_auth::controllers::AuthRegistrations::<#user>::#action_ident }
        }
        "passwords" => quote! { doido_auth::controllers::AuthPasswords::<#user>::#action_ident },
        "oauth" => quote! { doido_auth::controllers::AuthOauth::#action_ident },
        "two_factor" => quote! { doido_auth::controllers::AuthTwoFactor::<#user>::#action_ident },
        other => {
            let msg = format!("unknown auth module `{other}`");
            quote! { compile_error!(#msg) }
        }
    }
}

struct RouteSpec {
    method: &'static str,
    path: String,
    module: &'static str,
    action: &'static str,
    html_only: bool,
}

fn route_specs(prefix: &str) -> Vec<RouteSpec> {
    vec![
        RouteSpec {
            method: "GET",
            path: join_path(prefix, "sign_in"),
            module: "sessions",
            action: "new",
            html_only: true,
        },
        RouteSpec {
            method: "POST",
            path: join_path(prefix, "sign_in"),
            module: "sessions",
            action: "create",
            html_only: false,
        },
        RouteSpec {
            method: "DELETE",
            path: join_path(prefix, "sign_out"),
            module: "sessions",
            action: "destroy",
            html_only: false,
        },
        RouteSpec {
            method: "GET",
            path: join_path(prefix, "sign_up"),
            module: "registrations",
            action: "new",
            html_only: true,
        },
        RouteSpec {
            method: "POST",
            path: join_path(prefix, "sign_up"),
            module: "registrations",
            action: "create",
            html_only: false,
        },
        RouteSpec {
            method: "GET",
            path: join_path(prefix, "password/new"),
            module: "passwords",
            action: "new",
            html_only: true,
        },
        RouteSpec {
            method: "POST",
            path: join_path(prefix, "password"),
            module: "passwords",
            action: "create",
            html_only: false,
        },
        RouteSpec {
            method: "PATCH",
            path: join_path(prefix, "password"),
            module: "passwords",
            action: "update",
            html_only: false,
        },
        RouteSpec {
            method: "GET",
            path: "/auth/{provider}".into(),
            module: "oauth",
            action: "authorize",
            html_only: false,
        },
        RouteSpec {
            method: "GET",
            path: "/auth/{provider}/callback".into(),
            module: "oauth",
            action: "callback",
            html_only: false,
        },
        RouteSpec {
            method: "GET",
            path: join_path(prefix, "two_factor/new"),
            module: "two_factor",
            action: "new",
            html_only: true,
        },
        RouteSpec {
            method: "POST",
            path: join_path(prefix, "two_factor"),
            module: "two_factor",
            action: "create",
            html_only: false,
        },
        RouteSpec {
            method: "POST",
            path: join_path(prefix, "two_factor/challenge"),
            module: "two_factor",
            action: "challenge",
            html_only: false,
        },
    ]
}

pub fn expand_auth_routes(input: TokenStream, api_only: bool) -> TokenStream {
    match plan_auth_routes(input, api_only) {
        Ok((route_stmts, descriptors)) => {
            let entries = descriptors.iter().map(|(method, path)| {
                quote! {
                    ::doido_controller::RouteEntry {
                        method: #method.to_string(),
                        path: #path.to_string(),
                    }
                }
            });
            quote! {
                {
                    ::doido_controller::register_routes(::std::vec![ #(#entries),* ]);
                    doido_controller::axum::Router::new()
                    #route_stmts
                }
            }
        }
        Err(e) => e.to_compile_error(),
    }
}

/// Plan auth routes for embedding inside [`doido_controller::routes!`] (no
/// `register_routes` side effect — the parent macro owns the route table).
pub fn plan_auth_routes(
    input: TokenStream,
    api_only: bool,
) -> Result<(TokenStream, Vec<(String, String)>)> {
    let parsed: AuthRoutesInput = syn::parse2(input)?;
    let prefix = parsed.prefix.as_deref().unwrap_or("/users");
    let user = &parsed.user;

    let mut route_stmts = TokenStream::new();
    let mut descriptors = Vec::new();

    for spec in route_specs(prefix) {
        if !module_enabled(spec.module, &parsed.only, &parsed.skip) {
            continue;
        }
        if api_only && spec.html_only {
            continue;
        }
        if spec.module == "two_factor" && !cfg!(feature = "auth-2fa") {
            continue;
        }

        let handler = handler_for(
            spec.module,
            spec.action,
            user,
            &parsed.controllers,
            &parsed.actions,
        );
        let path = spec.path.clone();
        let method = spec.method;
        let axum_method = format_ident!("{}", method.to_lowercase());
        descriptors.push((method.to_string(), path.clone()));

        route_stmts.extend(quote! {
            .route(
                #path,
                doido_controller::axum::routing::#axum_method(#handler),
            )
        });
    }

    Ok((route_stmts, descriptors))
}

/// Expand auth routes as `get!`/`post!`/… declarations for embedding in
/// [`doido_auth::routes!`].
pub fn expand_auth_route_decls(input: TokenStream, api_only: bool) -> TokenStream {
    match expand_auth_route_decls_inner(input, api_only) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error(),
    }
}

fn expand_auth_route_decls_inner(input: TokenStream, api_only: bool) -> Result<TokenStream> {
    let parsed: AuthRoutesInput = syn::parse2(input)?;
    let prefix = parsed.prefix.as_deref().unwrap_or("/users");
    let user = &parsed.user;
    let mut decls = TokenStream::new();

    for spec in route_specs(prefix) {
        if !module_enabled(spec.module, &parsed.only, &parsed.skip) {
            continue;
        }
        if api_only && spec.html_only {
            continue;
        }
        if spec.module == "two_factor" && !cfg!(feature = "auth-2fa") {
            continue;
        }

        let handler = handler_for(
            spec.module,
            spec.action,
            user,
            &parsed.controllers,
            &parsed.actions,
        );
        let path = spec.path;
        let method = format_ident!("{}", spec.method.to_lowercase());
        decls.extend(quote! {
            #method!(#path, #handler);
        });
    }

    Ok(decls)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_devise_style_options() {
        let input: AuthRoutesInput = syn::parse2(quote! {
            User, only: [sessions, registrations], prefix: "/accounts",
            controllers: { sessions: auth::SessionsController },
            actions: { sessions: { create: auth::SessionsController::create } }
        })
        .unwrap();
        assert_eq!(input.user.to_string(), "User");
        assert_eq!(input.prefix.as_deref(), Some("/accounts"));
        assert_eq!(input.only.as_ref().unwrap().len(), 2);
        assert!(input.controllers.contains_key("sessions"));
    }
}
