use doido_controller::constraints::{alpha, numeric, uuid_like, Constraints};

#[test]
fn format_validators() {
    assert!(numeric("42"));
    assert!(!numeric("4a"));
    assert!(!numeric(""));
    assert!(alpha("abc"));
    assert!(!alpha("ab3"));
    assert!(uuid_like("550e8400-e29b-41d4-a716-446655440000"));
    assert!(!uuid_like("not-a-uuid"));
}

#[test]
fn constraints_match_named_params() {
    let c = Constraints::new().param("id", numeric);
    assert!(c.matches(&[("id", "42")]));
    assert!(!c.matches(&[("id", "abc")]), "id must be numeric");
    // a param not covered by a rule is ignored
    assert!(c.matches(&[("id", "7"), ("slug", "anything")]));
    // a required-by-rule param that is absent fails
    assert!(!c.matches(&[("other", "1")]));
}
