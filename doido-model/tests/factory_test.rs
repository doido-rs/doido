use doido_model::factory::{sequence, Factory};

struct User {
    email: String,
    #[allow(dead_code)]
    name: String,
}

impl Factory for User {
    fn build() -> Self {
        let n = sequence();
        User {
            email: format!("user{n}@example.com"),
            name: "Test User".into(),
        }
    }
}

#[test]
fn factory_builds_records_with_unique_sequences() {
    let a = User::build();
    let b = User::build();
    assert_ne!(a.email, b.email, "the sequence keeps records unique");
}

#[test]
fn build_list_builds_n_distinct_records() {
    let users = User::build_list(3);
    assert_eq!(users.len(), 3);
    let mut emails: Vec<_> = users.iter().map(|u| u.email.clone()).collect();
    emails.sort();
    emails.dedup();
    assert_eq!(emails.len(), 3, "all built records are distinct");
}

#[test]
fn sequence_is_monotonic() {
    let x = sequence();
    let y = sequence();
    assert!(y > x);
}
