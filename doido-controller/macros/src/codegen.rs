use crate::parser::{ResourceFilter, RouteDecl, RoutesInput};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};

fn is_active(action: &str, filter: &ResourceFilter) -> bool {
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
                route_stmts.push(quote! {
                    .route(#full_path, axum::routing::#axum_method(#handler))
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
                    .route(#full_path, axum::routing::get(#handler))
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

                let mut collection = quote! { axum::routing::MethodRouter::new() };
                if is_active("index", &filter) {
                    collection = quote! { #collection.get(#ctrl::index) };
                    descriptors.push(("GET".to_string(), base.clone()));
                }
                if is_active("create", &filter) {
                    collection = quote! { #collection.post(#ctrl::create) };
                    descriptors.push(("POST".to_string(), base.clone()));
                }
                route_stmts.push(quote! { .route(#base, #collection) });

                if is_active("new", &filter) {
                    descriptors.push(("GET".to_string(), base_new.clone()));
                    route_stmts.push(quote! { .route(#base_new, axum::routing::get(#ctrl::new)) });
                }

                let mut member = quote! { axum::routing::MethodRouter::new() };
                if is_active("show", &filter) {
                    member = quote! { #member.get(#ctrl::show) };
                    descriptors.push(("GET".to_string(), base_id.clone()));
                }
                if is_active("update", &filter) {
                    member = quote! { #member.patch(#ctrl::update).put(#ctrl::update) };
                    descriptors.push(("PUT|PATCH".to_string(), base_id.clone()));
                }
                if is_active("destroy", &filter) {
                    member = quote! { #member.delete(#ctrl::destroy) };
                    descriptors.push(("DELETE".to_string(), base_id.clone()));
                }
                route_stmts.push(quote! { .route(#base_id, #member) });

                if is_active("edit", &filter) {
                    descriptors.push(("GET".to_string(), base_id_edit.clone()));
                    route_stmts
                        .push(quote! { .route(#base_id_edit, axum::routing::get(#ctrl::edit)) });
                }

                // Collection routes: `GET /{resource}/<action>` (Rails `collection`).
                for action in &collection_actions {
                    let path = format!("{}/{}", base, action);
                    let action_ident = format_ident!("{}", action);
                    descriptors.push(("GET".to_string(), path.clone()));
                    route_stmts
                        .push(quote! { .route(#path, axum::routing::get(#ctrl::#action_ident)) });
                }
                // Member routes: `GET /{resource}/{id}/<action>` (Rails `member`).
                for action in &member_actions {
                    let path = format!("{}/{{id}}/{}", base, action);
                    let action_ident = format_ident!("{}", action);
                    descriptors.push(("GET".to_string(), path.clone()));
                    route_stmts
                        .push(quote! { .route(#path, axum::routing::get(#ctrl::#action_ident)) });
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
                let new_fn = format_ident!("new_{}_path", helper_singular);
                let member_fn = format_ident!("{}_path", helper_singular);
                let edit_fn = format_ident!("edit_{}_path", helper_singular);

                helper_fns.push(quote! {
                    #[allow(dead_code)]
                    fn #collection_fn() -> &'static str { #base }
                    #[allow(dead_code)]
                    fn #new_fn() -> &'static str { #base_new }
                    #[allow(dead_code)]
                    fn #member_fn(id: impl ::std::fmt::Display) -> String {
                        format!("{}/{}", #base, id)
                    }
                    #[allow(dead_code)]
                    fn #edit_fn(id: impl ::std::fmt::Display) -> String {
                        format!("{}/{}/edit", #base, id)
                    }
                });
            }
            RouteDecl::Redirect { from, to } => {
                let full_from = match path_prefix {
                    Some(pfx) => syn::LitStr::new(&format!("{}{}", pfx, from.value()), from.span()),
                    None => from,
                };
                descriptors.push(("GET".to_string(), full_from.value()));
                route_stmts.push(quote! {
                    .route(#full_from, axum::routing::get(|| async move {
                        axum::response::Response::builder()
                            .status(301)
                            .header(axum::http::header::LOCATION, #to)
                            .body(axum::body::Body::empty())
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
                descriptors.push(("GET".to_string(), base_new.clone()));
                descriptors.push(("GET".to_string(), base_edit.clone()));

                route_stmts.push(quote! {
                    .route(#base, axum::routing::MethodRouter::new()
                        .get(#ctrl::show)
                        .post(#ctrl::create)
                        .patch(#ctrl::update)
                        .put(#ctrl::update)
                        .delete(#ctrl::destroy))
                });
                route_stmts.push(quote! { .route(#base_new, axum::routing::get(#ctrl::new)) });
                route_stmts.push(quote! { .route(#base_edit, axum::routing::get(#ctrl::edit)) });

                let helper_base = match helper_prefix {
                    Some(pfx) => format!("{}_{}", pfx, n),
                    None => n.clone(),
                };
                let show_fn = format_ident!("{}_path", helper_base);
                let new_fn = format_ident!("new_{}_path", helper_base);
                let edit_fn = format_ident!("edit_{}_path", helper_base);
                helper_fns.push(quote! {
                    #[allow(dead_code)]
                    fn #show_fn() -> &'static str { #base }
                    #[allow(dead_code)]
                    fn #new_fn() -> &'static str { #base_new }
                    #[allow(dead_code)]
                    fn #edit_fn() -> &'static str { #base_edit }
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
                descriptors.push(("GET".to_string(), nested_new.clone()));
                descriptors.push(("GET".to_string(), member.clone()));
                descriptors.push(("PUT|PATCH".to_string(), member.clone()));
                descriptors.push(("DELETE".to_string(), member.clone()));
                descriptors.push(("GET".to_string(), member_edit.clone()));

                route_stmts.push(quote! {
                    .route(#nested, axum::routing::MethodRouter::new()
                        .get(#ctrl::index)
                        .post(#ctrl::create))
                });
                route_stmts.push(quote! { .route(#nested_new, axum::routing::get(#ctrl::new)) });
                route_stmts.push(quote! {
                    .route(#member, axum::routing::MethodRouter::new()
                        .get(#ctrl::show)
                        .patch(#ctrl::update)
                        .put(#ctrl::update)
                        .delete(#ctrl::destroy))
                });
                route_stmts.push(quote! { .route(#member_edit, axum::routing::get(#ctrl::edit)) });
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
                let inner_ts =
                    generate_inner(body, Some(&ns_path), Some(&combined_helper), descriptors);
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
                let inner_ts = generate_inner(body, Some(&full_path), helper_prefix, descriptors);
                route_stmts.push(quote! { .merge(#inner_ts) });
            }
        }
    }

    quote! {
        {
            #(#helper_fns)*
            axum::Router::new()
            #(#route_stmts)*
        }
    }
}

pub fn generate(input: RoutesInput) -> TokenStream {
    let mut descriptors = Vec::new();
    let inner = generate_inner(input, None, None, &mut descriptors);

    let entries = descriptors.iter().map(|(method, path)| {
        quote! {
            ::doido_controller::RouteEntry {
                method: #method.to_string(),
                path: #path.to_string(),
            }
        }
    });

    // Register the route table (for `doido server` / `doido routes` to print)
    // as the router is built, then yield the router itself.
    quote! {
        {
            ::doido_controller::register_routes(::std::vec![ #(#entries),* ]);
            #inner
        }
    }
}
