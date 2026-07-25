use doido_generators::commands::console::banner;

#[test]
fn console_banner_reports_environment_and_guidance() {
    let b = banner();
    assert!(b.contains("doido console"));
    assert!(b.to_lowercase().contains("evcxr"), "mentions the REPL: {b}");
}
