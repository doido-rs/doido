mod controller;

use proc_macro::TokenStream;

/// Marks an impl block as a controller. Rewrites action methods into
/// axum-compatible handlers with filter chain support.
#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    match controller::expand_controller(attr.into(), item.into()) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Registers a before-action filter on the following action method.
/// Usage: `#[before_action(fn_name)]` or `#[before_action(fn_name, only = [action1, action2])]`
#[proc_macro_attribute]
pub fn before_action(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Registers an after-action filter on the following action method.
/// Usage: `#[after_action(fn_name)]`
#[proc_macro_attribute]
pub fn after_action(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
