use doido_controller::session::{CookieSessionStore, Session, SessionStore};

struct FakeStore;

#[async_trait::async_trait]
impl SessionStore for FakeStore {
    async fn load(&self, _id: &str) -> doido_core::Result<Option<Session>> {
        Ok(None)
    }
    async fn save(&self, _session: &Session) -> doido_core::Result<()> {
        Ok(())
    }
    async fn destroy(&self, _id: &str) -> doido_core::Result<()> {
        Ok(())
    }
}

#[test]
fn test_session_store_trait_is_object_safe() {
    let _store: &dyn SessionStore = &FakeStore;
    // just checking it compiles as a trait object
}

#[test]
fn test_session_has_id_and_data() {
    let s = Session {
        id: "abc".to_string(),
        data: serde_json::json!({"k": "v"}),
    };
    assert_eq!(s.id, "abc");
}

#[test]
fn test_session_typed_get_set() {
    let mut s = Session::new();
    assert!(!s.id.is_empty(), "new session has a generated id");
    s.set("user_id", 42);
    s.set("role", "admin");
    assert_eq!(s.get::<i64>("user_id"), Some(42));
    assert_eq!(s.get::<String>("role"), Some("admin".to_string()));
    assert_eq!(s.get::<i64>("missing"), None);
}

#[test]
fn test_cookie_session_signed_round_trip() {
    let store = CookieSessionStore::new(b"super-secret-key".to_vec());
    let mut s = Session::new();
    s.set("user_id", 42);
    s.set("role", "admin");

    let cookie = store.encode(&s);
    let back = store
        .decode(&cookie)
        .expect("a valid signed cookie decodes");
    assert_eq!(back.get::<i64>("user_id"), Some(42));
    assert_eq!(back.get::<String>("role"), Some("admin".to_string()));
    assert_eq!(back.id, s.id, "session id survives the round trip");
}

#[test]
fn test_cookie_session_rejects_tampering() {
    let store = CookieSessionStore::new(b"super-secret-key".to_vec());
    let mut s = Session::new();
    s.set("admin", true);
    let cookie = store.encode(&s);

    // Flip the first character of the signed payload: the HMAC no longer matches.
    let mut chars: Vec<char> = cookie.chars().collect();
    chars[0] = if chars[0] == 'A' { 'B' } else { 'A' };
    let tampered: String = chars.into_iter().collect();

    assert!(
        store.decode(&tampered).is_none(),
        "a tampered cookie must not decode"
    );
}

#[test]
fn test_cookie_session_rejects_wrong_secret() {
    let signer = CookieSessionStore::new(b"key-a".to_vec());
    let attacker = CookieSessionStore::new(b"key-b".to_vec());
    let mut s = Session::new();
    s.set("x", 1);
    let cookie = signer.encode(&s);
    assert!(
        attacker.decode(&cookie).is_none(),
        "a cookie signed with a different secret must not decode"
    );
}
