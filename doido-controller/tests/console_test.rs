use doido_controller::commands::console::banner;

#[test]
fn console_banner_mentions_environment() {
    let text = banner();
    assert!(text.contains("doido console"));
}
