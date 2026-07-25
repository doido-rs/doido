use doido_view::helpers::asset::importmap;

#[test]
fn importmap_pins_modules() {
    let map = importmap(&[
        ("application", "/assets/application.js"),
        ("controllers", "/assets/controllers.js"),
    ]);
    assert!(map.contains("type=\"importmap\""));
    assert!(map.contains("\"application\":\"/assets/application.js\""));
    assert!(map.contains("\"controllers\":\"/assets/controllers.js\""));
}
