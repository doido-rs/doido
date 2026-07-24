//! The [`Mailer`] trait, implemented by the `#[mailer]` macro.
//!
//! It carries the snake_case mailer name derived from the struct, which drives
//! template resolution following the mailer convention in `docs/08-mailer.md`:
//! `mailers/<mailer_name>/<action>`.

/// A mailer type. The `#[mailer]` attribute generates the implementation from
/// the struct name; the struct's own action methods (which build [`Mail`] values)
/// are left untouched.
///
/// [`Mail`]: crate::Mail
pub trait Mailer {
    /// The snake_case name of this mailer (`UserMailer` → `"user_mailer"`),
    /// derived by the `#[mailer]` macro.
    fn mailer_name() -> &'static str;

    /// Template key for an action, following the mailer template convention:
    /// `mailers/<mailer_name>/<action>`.
    fn template_key(action: &str) -> String {
        format!("mailers/{}/{}", Self::mailer_name(), action)
    }
}
