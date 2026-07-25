use doido_cable::connection::CableConnection;

#[test]
fn connection_is_rejected_until_authorized() {
    let mut conn = CableConnection::new();
    assert!(!conn.is_authorized(), "rejected by default");

    conn.identify("current_user", "42").authorize();
    assert!(conn.is_authorized());
    assert_eq!(conn.identifier("current_user"), Some("42"));
    assert_eq!(conn.identifier("missing"), None);

    conn.reject();
    assert!(!conn.is_authorized());
}
