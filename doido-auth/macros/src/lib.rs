mod api_mode;
mod auth_controller;

use doido_auth_route_dsl::{expand_auth_route_decls, expand_auth_routes};
use proc_macro::TokenStream;

/// Standalone Devise-style auth router (`.merge(doido_auth::auth_routes!(User))`).
#[proc_macro]
pub fn auth_routes(input: TokenStream) -> TokenStream {
    expand_auth_routes(input.into(), api_mode::api_only()).into()
}

/// Internal: expands to `get!`/`post!`/… lines consumed by [`doido_auth::routes!`].
#[proc_macro]
pub fn __auth_routes_decl(input: TokenStream) -> TokenStream {
    expand_auth_route_decls(input.into(), api_mode::api_only()).into()
}

/// Marks an auth controller impl, preserving generic type parameters (unlike
/// `doido_controller::controller` on `impl<U> …`).
#[proc_macro_attribute]
pub fn auth_controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    match auth_controller::expand_auth_controller(attr.into(), item.into()) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
