pub mod channel;
pub mod controller;
pub mod field;
pub mod job;
pub mod mailer;
pub mod migration;
pub mod model;
pub mod new;
pub mod scaffold;

use doido_core::Inflector;

/// `BlogPost`/`blog-post` → `blog_post`.
pub fn to_snake(s: &str) -> String {
    Inflector::underscore(s)
}

/// `blog_post`/`blog-post` → `BlogPost`.
pub fn to_pascal(s: &str) -> String {
    // Normalise dashes/casing first so `camelize` (which splits on `_`) sees
    // clean snake_case input.
    Inflector::camelize(&Inflector::underscore(s))
}

/// `BlogPost`/`blog_post` → `blog_posts` — the pluralized, snake_cased table
/// name, honouring any custom rules from `config/inflection.yaml`.
pub fn to_table_name(s: &str) -> String {
    Inflector::tableize(s)
}
