//! Static [`Inflector`] facade over the global instance.

use doido_core::inflector::Inflector;

#[test]
fn static_facade_delegates_to_global_rules() {
    assert_eq!(Inflector::pluralize("post"), "posts");
    assert_eq!(Inflector::singularize("posts"), "post");
    assert_eq!(Inflector::camelize("post_comment"), "PostComment");
    assert_eq!(Inflector::camelize_lower("post_comment"), "postComment");
    assert_eq!(Inflector::underscore("PostComment"), "post_comment");
    assert_eq!(Inflector::dasherize("post_comment"), "post-comment");
    assert_eq!(Inflector::humanize("post_comment"), "Post comment");
    assert_eq!(Inflector::tableize("PostComment"), "post_comments");
    assert_eq!(Inflector::classify("post_comments"), "PostComment");
    assert_eq!(Inflector::foreign_key("PostComment"), "post_comment_id");
    assert_eq!(Inflector::constantize("post-comment"), "POST_COMMENT");
}
