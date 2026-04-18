use doido_view::engine::TemplateEngine;
use serde_json::json;

struct FakeEngine;
impl TemplateEngine for FakeEngine {
    fn render(&self, template: &str, _ctx: &serde_json::Value) -> doido_core::Result<String> {
        Ok(format!("rendered:{template}"))
    }
    fn reload(&self) -> doido_core::Result<()> { Ok(()) }
}

#[test]
fn test_engine_trait_is_object_safe() {
    let engine: &dyn TemplateEngine = &FakeEngine;
    let result = engine.render("posts/index", &json!({})).unwrap();
    assert_eq!(result, "rendered:posts/index");
}
