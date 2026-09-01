use doido_auth::AuthError;

fn assert_contains(err: AuthError, needle: &str) {
    let msg = err.to_string();
    assert!(msg.contains(needle), "expected `{needle}` in `{msg}`");
}

#[test]
fn auth_error_display_messages() {
    assert_contains(AuthError::InvalidCredentials, "invalid credentials");
    assert_contains(AuthError::EmailTaken, "email already taken");
    assert_contains(AuthError::Validation("too short".into()), "too short");
    assert_contains(AuthError::NotConfirmed, "email not confirmed");
    assert_contains(AuthError::AccountLocked, "account locked");
    assert_contains(AuthError::Unauthorized, "unauthorized");
    assert_contains(AuthError::InvalidToken, "invalid token");
    assert_contains(AuthError::Jwt("bad sig".into()), "bad sig");
    assert_contains(AuthError::OAuth("denied".into()), "denied");
    assert_contains(AuthError::Config("missing secret".into()), "missing secret");
    assert_contains(AuthError::UnknownStrategy("jwt".into()), "unknown strategy");
    assert_contains(AuthError::Internal("db down".into()), "db down");
}
