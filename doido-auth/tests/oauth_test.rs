//! OAuth provider tests.

use doido_auth::config::{OAuthProviderConfig, OAuthProviderType};
use doido_auth::oauth::{
    get_provider, register_provider, OAuth2Provider, OAuthProvider, OAuthTokenResponse,
};
use doido_auth::mount;
use doido_auth::testing::TestUser;
use doido_auth::testing::{init_test_auth, send, test_auth_config};
use doido_auth::AuthError;
use doido_model::testing::TestDb;
use http::StatusCode;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn oauth2_config(token_url: &str) -> OAuthProviderConfig {
    OAuthProviderConfig {
        provider_type: OAuthProviderType::Oauth2,
        client_id: Some("cid".into()),
        client_secret: Some("sec".into()),
        redirect_uri: Some("http://localhost/callback".into()),
        scopes: vec!["openid".into(), "email".into()],
        authorize_url: Some("https://example.com/oauth/authorize".into()),
        token_url: Some(token_url.into()),
        consumer_key: None,
        consumer_secret: None,
    }
}

struct StaticProvider {
    name: &'static str,
    authorize: String,
    token: OAuthTokenResponse,
}

impl OAuthProvider for StaticProvider {
    fn name(&self) -> &str {
        self.name
    }

    fn authorize_url(&self, state: &str) -> Result<String, AuthError> {
        Ok(format!("{}?state={state}", self.authorize))
    }

    fn exchange_code(&self, _code: &str) -> Result<OAuthTokenResponse, AuthError> {
        Ok(self.token.clone())
    }
}

#[test]
fn authorize_url_includes_client_and_state() {
    let provider = OAuth2Provider::new("example", oauth2_config("https://example.com/token"));
    let url = provider.authorize_url("state123").unwrap();
    assert!(url.contains("client_id=cid"));
    assert!(url.contains("state=state123"));
    assert!(url.contains("scope=openid"));
}

#[test]
fn from_config_rejects_non_oauth2_provider() {
    let mut cfg = oauth2_config("https://example.com/token");
    cfg.provider_type = OAuthProviderType::Oauth1;
    assert!(OAuth2Provider::from_config("legacy", cfg).is_err());
}

#[test]
fn authorize_url_requires_client_id() {
    let mut cfg = oauth2_config("https://example.com/token");
    cfg.client_id = None;
    let provider = OAuth2Provider::new("example", cfg);
    assert!(provider.authorize_url("s").is_err());
}

#[test]
fn register_and_get_provider_via_trait_object() {
    let provider = Arc::new(OAuth2Provider::new(
        "example",
        oauth2_config("https://example.com/token"),
    )) as Arc<dyn OAuthProvider>;
    register_provider(provider);
    assert!(get_provider("example").is_some());
}

#[test]
fn custom_provider_implements_trait() {
    let provider = Arc::new(StaticProvider {
        name: "custom",
        authorize: "https://idp.example/auth".into(),
        token: OAuthTokenResponse {
            access_token: "custom-tok".into(),
            token_type: Some("Bearer".into()),
            refresh_token: None,
            expires_in: None,
            id_token: None,
        },
    });
    register_provider(provider);
    let found = get_provider("custom").expect("registered");
    assert_eq!(
        found.authorize_url("abc").unwrap(),
        "https://idp.example/auth?state=abc"
    );
    assert_eq!(
        found.exchange_code("code").unwrap().access_token,
        "custom-tok"
    );
}

#[test]
fn providers_from_config_skips_oauth1() {
    let mut oauth = HashMap::new();
    oauth.insert(
        "legacy".into(),
        OAuthProviderConfig {
            provider_type: OAuthProviderType::Oauth1,
            client_id: None,
            client_secret: None,
            consumer_key: None,
            consumer_secret: None,
            redirect_uri: None,
            scopes: vec![],
            authorize_url: None,
            token_url: None,
        },
    );
    oauth.insert("example".into(), oauth2_config("https://example.com/token"));
    let config = doido_auth::AuthConfig {
        oauth,
        ..Default::default()
    };
    let providers = doido_auth::oauth::providers_from_config(&config.oauth);
    assert_eq!(providers.len(), 1);
    assert!(providers.contains_key("example"));
}

fn spawn_token_server(body: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.set_nonblocking(true).expect("set_nonblocking");
    let addr = listener.local_addr().unwrap();
    let body = body.to_string();
    thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if std::time::Instant::now() > deadline {
                return;
            }
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
    });
    format!("http://{addr}/token")
}

#[test]
fn exchange_code_returns_tokens_from_provider() {
    let json = r#"{"access_token":"tok123","token_type":"Bearer"}"#;
    let token_url = spawn_token_server(json);
    let provider = OAuth2Provider::new("mock", oauth2_config(&token_url));
    let tokens = provider.exchange_code("auth-code").unwrap();
    assert_eq!(tokens.access_token, "tok123");
}

#[test]
fn exchange_code_reports_http_errors() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if std::time::Instant::now() > deadline {
                return;
            }
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let resp = "HTTP/1.1 400 Bad Request\r\nContent-Length: 5\r\nConnection: close\r\n\r\nerror";
                let _ = stream.write_all(resp.as_bytes());
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
    });
    let provider = OAuth2Provider::new("mock", oauth2_config(&format!("http://{addr}/token")));
    assert!(provider.exchange_code("bad").is_err());
}

#[tokio::test]
async fn oauth_redirect_and_callback_via_routes() {
    let json = r#"{"access_token":"oauth-tok","token_type":"Bearer"}"#;
    let token_url = spawn_token_server(json);
    let db = TestDb::new().await.unwrap();
    let mut config = test_auth_config();
    config
        .oauth
        .insert("mock".into(), oauth2_config(&token_url));
    let _auth = init_test_auth(db.conn().clone(), config).await.unwrap();

    let app = mount::<TestUser, _>(|_db, _email, _digest| Box::pin(async { panic!("not used") }));

    let redirect = send(app.clone(), "GET", "/auth/mock", "").await;
    assert_eq!(redirect.status, StatusCode::TEMPORARY_REDIRECT);

    let callback = send(app, "GET", "/auth/mock/callback?code=abc&state=xyz", "").await;
    assert_eq!(callback.status, StatusCode::OK);
    assert!(callback.body.contains("oauth-tok"));
}
