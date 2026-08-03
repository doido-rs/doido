//! Name inflection helpers — same conventions as `doido-generators`.

use doido_core::Inflector;

/// `BlogPost`/`blog-post` → `blog_post`.
pub fn to_snake(s: &str) -> String {
    Inflector::underscore(s)
}

/// `blog_post`/`blog-post` → `BlogPost`.
pub fn to_pascal(s: &str) -> String {
    Inflector::camelize(&Inflector::underscore(s))
}

/// `BlogPost`/`blog_post` → `blog_posts`.
pub fn to_table_name(s: &str) -> String {
    Inflector::tableize(s)
}
