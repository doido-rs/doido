mod codegen;
mod controller;
mod parser;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Expands the `routes!` DSL (verbs, `resources!`, `namespace!`, `scope!`)
/// into an `axum::Router`. Merged in from the former `doido-router` crate.
#[proc_macro]
pub fn routes(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as parser::RoutesInput);
    codegen::generate(parsed).into()
}

/// Marks an impl block as a controller. Rewrites action methods into
/// axum-compatible handlers, wiring in any filters.
///
/// Filters are declared with the `#[before_action(...)]` / `#[after_action(...)]`
/// **helper attributes** on action methods; this macro parses and consumes them
/// while expanding the impl block, so there are no standalone
/// `before_action`/`after_action` macros to import:
///
/// ```ignore
/// #[controller]
/// impl PostsController {
///     #[before_action(require_auth)]
///     #[before_action(load_record, only = [show, edit])]
///     #[after_action(log_response)]
///     async fn show(ctx: &mut Context) -> Response { /* ... */ }
/// }
/// ```
///
/// `before_action` filters run in declaration order before the action and may
/// short-circuit by returning `Err(response)`; `after_action` filters run after.
#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    match controller::expand_controller(attr.into(), item.into()) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
