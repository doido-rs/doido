use doido_mailer::preview::MailerPreviews;
use doido_mailer::Mail;

fn welcome_preview() -> Mail {
    Mail::new()
        .to("preview@example.com")
        .subject("Welcome")
        .body_html("<h1>Hi there</h1>")
}

#[test]
fn previews_register_list_and_render() {
    let mut previews = MailerPreviews::new();
    previews.register("user_mailer/welcome", welcome_preview);

    assert_eq!(previews.names(), vec!["user_mailer/welcome"]);

    let rendered = previews.render("user_mailer/welcome").unwrap();
    assert!(rendered.contains("Subject: Welcome"));
    assert!(rendered.contains("<h1>Hi there</h1>"));

    assert!(previews.render("nope").is_none());
}
