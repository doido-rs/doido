//! Controller helpers — auxiliary modules imported by controllers.
//!
//! Helpers live in `app/helpers/` (e.g. `posts_helper.rs` with `PostsHelper`).
//! Mark the struct with `#[helper]` and call its methods from controller actions.

/// A controller helper type. The `#[helper]` attribute generates the implementation
/// from the struct name (`PostsHelper` → `"posts_helper"`).
pub trait Helper {
    /// Snake-case helper name used for convention-based lookup.
    fn helper_name() -> &'static str;
}
