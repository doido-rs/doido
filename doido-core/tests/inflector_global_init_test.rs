//! Global inflection init — isolated binary (`OnceLock`).

#[test]
fn init_inflections_installs_custom_rules() {
    doido_core::inflector::init_inflections(|i| {
        i.irregular("ox", "oxen");
    });
    assert_eq!(doido_core::inflector::Inflector::pluralize("ox"), "oxen");
}
