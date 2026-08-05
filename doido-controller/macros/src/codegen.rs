use crate::parser::{ResourceFilter, RouteDecl, RoutesInput};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};

fn is_active(action: &str, filter: &ResourceFilter, api: bool) -> bool {
    // In API-only projects the HTML-form routes (`new`/`edit`) are pointless —
    // they exist only to render forms — so they are always inactive.
    if api && matches!(action, "new" | "edit") {
        return false;
    }
    match filter {
        ResourceFilter::All => true,
        ResourceFilter::Only(list) => list.iter().any(|a| a == action),
        ResourceFilter::Except(list) => !list.iter().any(|a| a == action),
    }
}

fn generate_inner(
    input: RoutesInput,
    path_prefix: Option<&str>,
    helper_prefix: Option<&str>,
    descriptors: &mut Vec<(String, String)>,
    api: bool,
) -> TokenStream {
    let mut route_stmts = Vec::new();
    let mut helper_fns = Vec::new();

    for decl in input.decls {
        match decl {
            RouteDecl::Method {
                method,
                path,
                handler,
                name,
                constraints,
            } => {
                let axum_method = syn::Ident::new(&method, Span::call_site());
                let full_path = match path_prefix {
                    Some(pfx) => {
                        let combined = format!("{}{}", pfx, path.value());
                        syn::LitStr::new(&combined, path.span())
                    }
                    None => path,
                };
                descriptors.push((method.to_uppercase(), full_path.value()));
                let path_value = full_path.value();
                let method_router = if constraints.is_empty() {
                    quote! { doido_controller::axum::routing::#axum_method(#handler) }
                } else {
                    // Each `param: validator` becomes a check against the matched
                    // path params; any miss/failure 404s before the handler runs.
                    let checks = constraints.iter().map(|(param, validator)| {
                        let param_str = param.to_string();
                        quote! {
                            match __params.iter().find(|(k, _)| *k == #param_str).map(|(_, v)| v) {
                                Some(__v) if (#validator)(__v) => {}
                                _ => __ok = false,
                            }
                        }
                    });
                    quote! {
                        doido_controller::axum::routing::#axum_method(#handler).layer(doido_controller::axum::middleware::from_fn(
                            |__req: doido_controller::axum::extract::Request, __next: doido_controller::axum::middleware::Next| async move {
                                use doido_controller::axum::extract::FromRequestParts as _;
                                let (mut __parts, __body) = __req.into_parts();
                                let __ok = match doido_controller::axum::extract::RawPathParams::from_request_parts(&mut __parts, &()).await {
                                    Ok(__params) => {
                                        let mut __ok = true;
                                        #(#checks)*
                                        __ok
                                    }
                                    Err(_) => false,
                                };
                                let __req = doido_controller::axum::extract::Request::from_parts(__parts, __body);
                                if __ok {
                                    __next.run(__req).await
                                } else {
                                    doido_controller::axum::response::IntoResponse::into_response(
                                        doido_controller::axum::http::StatusCode::NOT_FOUND,
                                    )
                                }
                            },
                        ))
                    }
                };
                route_stmts.push(quote! {
                    .route(#full_path, #method_router)
                });
                // `as: name` → a `{name}_path()` helper returning the full path.
                if let Some(name) = name {
                    let helper_fn = format_ident!("{}_path", name);
                    helper_fns.push(quote! {
                        #[allow(dead_code)]
                        fn #helper_fn() -> &'static str { #path_value }
                    });
                }
            }
            RouteDecl::Root { handler } => {
                let full_path = match path_prefix {
                    Some(pfx) if !pfx.is_empty() => pfx.to_string(),
                    _ => "/".to_string(),
                };
                descriptors.push(("GET".to_string(), full_path.clone()));
                route_stmts.push(quote! {
                    .route(#full_path, doido_controller::axum::routing::get(#handler))
                });
                helper_fns.push(quote! {
                    #[allow(dead_code)]
                    fn root_path() -> &'static str { #full_path }
                });
            }
            RouteDecl::Resources {
                resource_name,
                controller,
                filter,
                member: member_actions,
                collection: collection_actions,
            } => {
                let name = resource_name.to_string();
                let singular = name.trim_end_matches('s').to_string();
                let prefix = path_prefix.unwrap_or("");
                let base = format!("{}/{}", prefix, name);
                let base_new = format!("{}/{}/new", prefix, name);
                let base_id = format!("{}/{}/{{id}}", prefix, name);
                let base_id_edit = format!("{}/{}/{{id}}/edit", prefix, name);
                let ctrl = &controller;

                let mut collection =
                    quote! { doido_controller::axum::routing::MethodRouter::new() };
                if is_active("index", &filter, api) {
                    collection = quote! { #collection.get(#ctrl::index) };
                    descriptors.push(("GET".to_string(), base.clone()));
                }
                if is_active("create", &filter, api) {
                    collection = quote! { #collection.post(#ctrl::create) };
                    descriptors.push(("POST".to_string(), base.clone()));
                }
                route_stmts.push(quote! { .route(#base, #collection) });

                let new_active = is_active("new", &filter, api);
                if new_active {
                    descriptors.push(("GET".to_string(), base_new.clone()));
                    route_stmts.push(quote! { .route(#base_new, doido_controller::axum::routing::get(#ctrl::new)) });
                }

                let mut member = quote! { doido_controller::axum::routing::MethodRouter::new() };
                if is_active("show", &filter, api) {
                    member = quote! { #member.get(#ctrl::show) };
                    descriptors.push(("GET".to_string(), base_id.clone()));
                }
                if is_active("update", &filter, api) {
                    member = quote! { #member.patch(#ctrl::update).put(#ctrl::update) };
                    descriptors.push(("PUT|PATCH".to_string(), base_id.clone()));
                }
                if is_active("destroy", &filter, api) {
                    member = quote! { #member.delete(#ctrl::destroy) };
                    descriptors.push(("DELETE".to_string(), base_id.clone()));
                }
                route_stmts.push(quote! { .route(#base_id, #member) });

                let edit_active = is_active("edit", &filter, api);
                if edit_active {
                    descriptors.push(("GET".to_string(), base_id_edit.clone()));
                    route_stmts
                        .push(quote! { .route(#base_id_edit, doido_controller::axum::routing::get(#ctrl::edit)) });
                }

                // Collection routes: `GET /{resource}/<action>` (Rails `collection`).
                for action in &collection_actions {
                    let path = format!("{}/{}", base, action);
                    let action_ident = format_ident!("{}", action);
                    descriptors.push(("GET".to_string(), path.clone()));
                    route_stmts
                        .push(quote! { .route(#path, doido_controller::axum::routing::get(#ctrl::#action_ident)) });
                }
                // Member routes: `GET /{resource}/{id}/<action>` (Rails `member`).
                for action in &member_actions {
                    let path = format!("{}/{{id}}/{}", base, action);
                    let action_ident = format_ident!("{}", action);
                    descriptors.push(("GET".to_string(), path.clone()));
                    route_stmts
                        .push(quote! { .route(#path, doido_controller::axum::routing::get(#ctrl::#action_ident)) });
                }

                // URL helpers with optional prefix
                let helper_name = match helper_prefix {
                    Some(pfx) => format!("{}_{}", pfx, name),
                    None => name.clone(),
                };
                let helper_singular = match helper_prefix {
                    Some(pfx) => format!("{}_{}", pfx, singular),
                    None => singular.clone(),
                };

                let collection_fn = format_ident!("{}_path", helper_name);
                let member_fn = format_ident!("{}_path", helper_singular);

                // `new_*_path` / `edit_*_path` helpers only exist when their routes
                // do (dropped in API mode along with the form routes).
                let new_helper = if new_active {
                    let new_fn = format_ident!("new_{}_path", helper_singular);
                    quote! {
                        #[allow(dead_code)]
                        fn #new_fn() -> &'static str { #base_new }
                    }
                } else {
                    quote! {}
                };
                let edit_helper = if edit_active {
                    let edit_fn = format_ident!("edit_{}_path", helper_singular);
                    quote! {
                        #[allow(dead_code)]
                        fn #edit_fn(id: impl ::std::fmt::Display) -> String {
                            format!("{}/{}/edit", #base, id)
                        }
                    }
                } else {
                    quote! {}
                };

                helper_fns.push(quote! {
                    #[allow(dead_code)]
                    fn #collection_fn() -> &'static str { #base }
                    #new_helper
                    #[allow(dead_code)]
                    fn #member_fn(id: impl ::std::fmt::Display) -> String {
                        format!("{}/{}", #base, id)
                    }
                    #edit_helper
                });
            }
            RouteDecl::Redirect { from, to } => {
                let full_from = match path_prefix {
                    Some(pfx) => syn::LitStr::new(&format!("{}{}", pfx, from.value()), from.span()),
                    None => from,
                };
                descriptors.push(("GET".to_string(), full_from.value()));
                route_stmts.push(quote! {
                    .route(#full_from, doido_controller::axum::routing::get(|| async move {
                        doido_controller::axum::response::Response::builder()
                            .status(301)
                            .header(doido_controller::axum::http::header::LOCATION, #to)
                            .body(doido_controller::axum::body::Body::empty())
                            .expect("valid redirect response")
                    }))
                });
            }
            RouteDecl::Mount { path, router } => {
                let full = match path_prefix {
                    Some(pfx) => format!("{}{}", pfx, path.value()),
                    None => path.value(),
                };
                descriptors.push(("MOUNT".to_string(), full.clone()));
                route_stmts.push(quote! { .nest(#full, #router) });
            }
            RouteDecl::Resource { name, controller } => {
                let n = name.to_string();
                let prefix = path_prefix.unwrap_or("");
                let base = format!("{}/{}", prefix, n); // e.g. /profile — no {id}
                let base_new = format!("{}/{}/new", prefix, n);
                let base_edit = format!("{}/{}/edit", prefix, n);
                let ctrl = &controller;

                descriptors.push(("GET".to_string(), base.clone()));
                descriptors.push(("POST".to_string(), base.clone()));
                descriptors.push(("PUT|PATCH".to_string(), base.clone()));
                descriptors.push(("DELETE".to_string(), base.clone()));

                route_stmts.push(quote! {
                    .route(#base, doido_controller::axum::routing::MethodRouter::new()
                        .get(#ctrl::show)
                        .post(#ctrl::create)
                        .patch(#ctrl::update)
                        .put(#ctrl::update)
                        .delete(#ctrl::destroy))
                });

                let helper_base = match helper_prefix {
                    Some(pfx) => format!("{}_{}", pfx, n),
                    None => n.clone(),
                };
                let show_fn = format_ident!("{}_path", helper_base);

                // API-only: drop the `new`/`edit` form routes and their helpers.
                let (new_edit_routes, new_edit_helpers) = if api {
                    (quote! {}, quote! {})
                } else {
                    descriptors.push(("GET".to_string(), base_new.clone()));
                    descriptors.push(("GET".to_string(), base_edit.clone()));
                    let new_fn = format_ident!("new_{}_path", helper_base);
                    let edit_fn = format_ident!("edit_{}_path", helper_base);
                    (
                        quote! {
                            .route(#base_new, doido_controller::axum::routing::get(#ctrl::new))
                            .route(#base_edit, doido_controller::axum::routing::get(#ctrl::edit))
                        },
                        quote! {
                            #[allow(dead_code)]
                            fn #new_fn() -> &'static str { #base_new }
                            #[allow(dead_code)]
                            fn #edit_fn() -> &'static str { #base_edit }
                        },
                    )
                };
                route_stmts.push(new_edit_routes);
                helper_fns.push(quote! {
                    #[allow(dead_code)]
                    fn #show_fn() -> &'static str { #base }
                    #new_edit_helpers
                });
            }
            RouteDecl::ShallowResources {
                parent,
                child,
                controller,
            } => {
                let prefix = path_prefix.unwrap_or("");
                let parent_s = parent.to_string();
                let parent_singular = parent_s.trim_end_matches('s');
                let child_s = child.to_string();
                let ctrl = &controller;

                // Collection routes stay nested under the parent.
                let nested = format!(
                    "{}/{}/{{{}_id}}/{}",
                    prefix, parent_s, parent_singular, child_s
                );
                let nested_new = format!("{}/new", nested);
                // Member routes drop the parent (the shallow part).
                let member = format!("{}/{}/{{id}}", prefix, child_s);
                let member_edit = format!("{}/edit", member);

                descriptors.push(("GET".to_string(), nested.clone()));
                descriptors.push(("POST".to_string(), nested.clone()));
                if !api {
                    descriptors.push(("GET".to_string(), nested_new.clone()));
                }
                descriptors.push(("GET".to_string(), member.clone()));
                descriptors.push(("PUT|PATCH".to_string(), member.clone()));
                descriptors.push(("DELETE".to_string(), member.clone()));
                if !api {
                    descriptors.push(("GET".to_string(), member_edit.clone()));
                }

                route_stmts.push(quote! {
                    .route(#nested, doido_controller::axum::routing::MethodRouter::new()
                        .get(#ctrl::index)
                        .post(#ctrl::create))
                });
                if !api {
                    route_stmts.push(quote! { .route(#nested_new, doido_controller::axum::routing::get(#ctrl::new)) });
                }
                route_stmts.push(quote! {
                    .route(#member, doido_controller::axum::routing::MethodRouter::new()
                        .get(#ctrl::show)
                        .patch(#ctrl::update)
                        .put(#ctrl::update)
                        .delete(#ctrl::destroy))
                });
                if !api {
                    route_stmts.push(quote! { .route(#member_edit, doido_controller::axum::routing::get(#ctrl::edit)) });
                }
            }
            RouteDecl::Namespace { name, body } => {
                let ns_str = name.to_string();
                let ns_path = match path_prefix {
                    Some(pfx) => format!("{}/{}", pfx, ns_str),
                    None => format!("/{}", ns_str),
                };
                let combined_helper = match helper_prefix {
                    Some(pfx) => format!("{}_{}", pfx, ns_str),
                    None => ns_str,
                };
                let inner_ts = generate_inner(
                    body,
                    Some(&ns_path),
                    Some(&combined_helper),
                    descriptors,
                    api,
                );
                route_stmts.push(quote! { .merge(#inner_ts) });
            }
            RouteDecl::Scope {
                path_prefix: scope_path,
                body,
            } => {
                let full_path = match path_prefix {
                    Some(pfx) => format!("{}{}", pfx, scope_path.value()),
                    None => scope_path.value(),
                };
                let inner_ts =
                    generate_inner(body, Some(&full_path), helper_prefix, descriptors, api);
                route_stmts.push(quote! { .merge(#inner_ts) });
            }
        }
    }

    quote! {
        {
            #(#helper_fns)*
            doido_controller::axum::Router::new()
            #(#route_stmts)*
        }
    }
}

pub fn generate(input: RoutesInput) -> TokenStream {
    let api = crate::api_mode::api_only();
    let mut descriptors = Vec::new();
    let inner = generate_inner(input, None, None, &mut descriptors, api);

    let entries = descriptors.iter().map(|(method, path)| {
        quote! {
            ::doido_controller::RouteEntry {
                method: #method.to_string(),
                path: #path.to_string(),
            }
        }
    });

    // Let rustc re-expand this macro when the `api_only` marker changes: stable
    // proc-macros don't track `fs::read`, but `include_bytes!` registers the file
    // as a compilation dependency. Only emitted when the file actually exists.
    let marker_tracking = match crate::api_mode::manifest_config_path() {
        Some(path) => {
            let path_str = path.to_string_lossy().into_owned();
            quote! { const _: &[u8] = include_bytes!(#path_str); }
        }
        None => quote! {},
    };

    // Register the route table (for `doido server` / `doido routes` to print)
    // as the router is built, then yield the router itself.
    quote! {
        {
            #marker_tracking
            ::doido_controller::register_routes(::std::vec![ #(#entries),* ]);
            #inner
        }
    }
}
