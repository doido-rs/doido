use doido_jobs::JobContext;

struct AppConfig {
    name: String,
}

#[test]
fn context_carries_typed_app_state() {
    let mut ctx = JobContext::new();
    ctx.insert(AppConfig {
        name: "doido".into(),
    });
    ctx.insert(42u32);

    assert_eq!(ctx.get::<AppConfig>().unwrap().name, "doido");
    assert_eq!(*ctx.get::<u32>().unwrap(), 42);
    assert!(ctx.get::<String>().is_none(), "unregistered type is None");
}
