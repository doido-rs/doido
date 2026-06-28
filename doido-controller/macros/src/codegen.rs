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
                route_stmts.push(quote! {
                    .route(#full_path, axum::routing::#axum_method(#handler))
                });
            }
            RouteDecl::Resources {
                resource_name,
                controller,
                filter,
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
