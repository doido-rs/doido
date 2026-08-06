mod api_mode;

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
