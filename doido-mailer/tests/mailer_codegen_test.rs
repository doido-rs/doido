use doido_mailer::mailer;

#[mailer]
pub struct UserMailer;

#[test]
fn mailer_macro_generates_inherent_name_and_template_key() {
    // Callable without importing the Mailer trait.
    assert_eq!(UserMailer::mailer_name(), "user_mailer");
    assert_eq!(
        UserMailer::template_key("welcome"),
        "mailers/user_mailer/welcome"
    );
}
