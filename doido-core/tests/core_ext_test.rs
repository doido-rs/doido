use doido_core::core_ext::{Blank, StringExt};

#[test]
fn blank_and_present() {
    assert!("".is_blank());
    assert!("   ".is_blank());
    assert!("x".is_present());
    assert!(String::new().is_blank());

    let none: Option<&str> = None;
    assert!(none.is_blank());
    assert!(Some("").is_blank());
    assert!(Some("x").is_present());

    assert!(Vec::<i32>::new().is_blank());
    assert!(vec![1, 2].is_present());
}

#[test]
fn truncate_chars_appends_ellipsis() {
    assert_eq!("hello".truncate_chars(10), "hello");
    assert_eq!("hello world".truncate_chars(5), "hell…");
}
