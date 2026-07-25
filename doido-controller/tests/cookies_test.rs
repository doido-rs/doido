use doido_controller::cookies::CookieJar;

fn value_of(headers: &[String], name: &str) -> String {
    let prefix = format!("{name}=");
    headers
        .iter()
        .find(|h| h.starts_with(&prefix))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .strip_prefix(&prefix)
        .unwrap()
        .to_string()
}

#[test]
fn reads_incoming_cookies() {
    let jar = CookieJar::from_header(Some("a=1; b=two"), b"k".to_vec());
    assert_eq!(jar.get("a"), Some("1"));
    assert_eq!(jar.get("b"), Some("two"));
    assert_eq!(jar.get("missing"), None);
}

#[test]
fn writes_set_cookie_headers() {
    let mut jar = CookieJar::from_header(None, b"k".to_vec());
    jar.set("theme", "dark");
    let headers = jar.to_set_cookie_headers();
    assert!(headers.iter().any(|h| h.starts_with("theme=dark")));
}

#[test]
fn signed_cookie_round_trips() {
    let secret = b"k".to_vec();
    let mut jar = CookieJar::from_header(None, secret.clone());
    jar.set_signed("uid", "42");
    let value = value_of(&jar.to_set_cookie_headers(), "uid");

    // The browser sends the signed value back on the next request.
    let next = CookieJar::from_header(Some(&format!("uid={value}")), secret);
    assert_eq!(next.get_signed("uid"), Some("42".to_string()));
    // A signed cookie is not readable as a plain one (it carries the signature).
    assert_ne!(next.get("uid"), Some("42"));
}

#[test]
fn tampered_signed_cookie_is_rejected() {
    let secret = b"k".to_vec();
    let mut jar = CookieJar::from_header(None, secret.clone());
    jar.set_signed("uid", "42");
    let value = value_of(&jar.to_set_cookie_headers(), "uid");

    let tampered = format!("uid={value}x");
    let next = CookieJar::from_header(Some(&tampered), secret);
    assert!(next.get_signed("uid").is_none());
}

#[test]
fn plain_cookie_is_not_accepted_as_signed() {
    let jar = CookieJar::from_header(Some("uid=42"), b"k".to_vec());
    assert!(jar.get_signed("uid").is_none());
}
