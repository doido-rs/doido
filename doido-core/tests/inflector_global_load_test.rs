//! Global inflection load from file — isolated binary (`OnceLock`).

#[test]
fn load_valid_yaml_applies_custom_rules() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("inflection.yaml");
    std::fs::write(
        &path,
        "irregulars:\n  - { singular: moose, plural: meese }\n",
    )
    .unwrap();
    let found = doido_core::load_inflections(&path).expect("valid yaml loads");
    assert!(found);
    assert_eq!(
        doido_core::inflector::Inflector::pluralize("moose"),
        "meese"
    );
}
