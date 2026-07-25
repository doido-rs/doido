use doido_controller::flash::Flash;
use doido_controller::session::CookieSessionStore;

#[test]
fn flash_round_trips_to_the_next_request() {
    let store = CookieSessionStore::new(b"secret".to_vec());
    let mut flash = Flash::new();
    flash.set("notice", "Saved!");
    flash.set("alert", "Careful");

    // The response writes a signed flash cookie; the next request reads it.
    let cookie = flash.to_cookie(&store);
    let next = Flash::from_cookie(&store, &cookie);
    assert_eq!(next.get("notice"), Some("Saved!"));
    assert_eq!(next.get("alert"), Some("Careful"));
}

#[test]
fn flash_does_not_persist_without_the_cookie() {
    let store = CookieSessionStore::new(b"secret".to_vec());
    // The request after next carries no flash cookie.
    let after = Flash::from_cookie(&store, "");
    assert!(
        after.is_empty(),
        "a flash lives for exactly one following request"
    );
}

#[test]
fn tampered_or_wrong_secret_flash_is_dropped() {
    let signer = CookieSessionStore::new(b"secret".to_vec());
    let attacker = CookieSessionStore::new(b"other".to_vec());
    let mut flash = Flash::new();
    flash.set("notice", "x");
    let cookie = flash.to_cookie(&signer);
    assert!(
        Flash::from_cookie(&attacker, &cookie).is_empty(),
        "an unverified flash cookie is ignored"
    );
}
