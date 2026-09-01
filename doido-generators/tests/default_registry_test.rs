use doido_generators::default_registry;

#[test]
fn default_registry_lists_core_generators() {
    let reg = default_registry();
    let names = reg.list();
    for expected in [
        "controller",
        "model",
        "migration",
        "scaffold",
        "new",
        "storage:install",
    ] {
        assert!(
            names.iter().any(|n| *n == expected),
            "missing generator `{expected}` in {names:?}"
        );
    }
}

#[test]
fn model_generator_is_registered() {
    let reg = default_registry();
    assert!(reg.list().contains(&"model"));
}
