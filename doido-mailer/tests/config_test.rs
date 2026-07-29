use doido_mailer::config::{Backend, MailerConfig, MailerFileConfig};
use doido_mailer::{Deliverer, LogDeliverer, Mail};
use std::sync::Arc;

#[test]
fn build_log_and_test_deliverers() {
    let log_cfg = MailerConfig {
        backend: Backend::Log,
        ..MailerConfig::default()
    };
    let _: Arc<dyn Deliverer> = log_cfg.build();

    let test_cfg = MailerConfig {
        backend: Backend::Test,
        ..MailerConfig::default()
    };
    let _deliverer: Arc<dyn Deliverer> = test_cfg.build();
}

#[tokio::test]
async fn build_test_deliverer_captures_mail() {
    let cfg = MailerConfig {
        backend: Backend::Test,
        ..MailerConfig::default()
    };
    let deliverer = cfg.build();
    let mail = Mail::new().to("a@b.com").subject("Hi");
    deliverer.deliver(&mail).await.unwrap();
}

#[test]
fn build_sendmail_deliverer() {
    let cfg = MailerConfig {
        backend: Backend::Sendmail,
        ..MailerConfig::default()
    };
    let _deliverer: Arc<dyn Deliverer> = cfg.build();
}

#[test]
fn build_smtp_with_starttls_and_insecure_auth() {
    let yaml = r#"
mailer:
  type: smtp
  smtp:
    address: mail.example:587
    starttls: true
    username: user
    password: pass
    allow_insecure_auth: true
"#;
    let cfg = MailerFileConfig::from_yaml(yaml)
        .unwrap()
        .mailer
        .into_config();
    let deliverer: Arc<dyn Deliverer> = cfg.build();
    let _ = deliverer;
}

#[test]
fn parses_sendmail_and_test_backends() {
    let sendmail = MailerFileConfig::from_yaml("mailer:\n  type: sendmail\n")
        .unwrap()
        .mailer
        .into_config();
    assert_eq!(sendmail.backend, Backend::Sendmail);

    let test = MailerFileConfig::from_yaml("mailer:\n  type: test\n")
        .unwrap()
        .mailer
        .into_config();
    assert_eq!(test.backend, Backend::Test);
}

#[test]
fn load_without_config_file_defaults_to_log() {
    let cfg = doido_mailer::config::load();
    assert_eq!(cfg.backend, Backend::Log);
}

#[tokio::test]
async fn log_deliverer_smoke() {
    let mail = Mail::new().to("x@y.com").subject("s");
    LogDeliverer.deliver(&mail).await.unwrap();
}
