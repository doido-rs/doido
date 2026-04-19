use serde_json::Value;

pub struct ViewResponse {
    pub template: String,
    pub context: Value,
    pub status: u16,
    pub layout: Option<String>,
}

impl ViewResponse {
    pub fn new(template: impl Into<String>, context: Value) -> Self {
        Self {
            template: template.into(),
            context,
            status: 200,
            layout: None,
        }
    }

    pub fn status(mut self, code: u16) -> Self {
        self.status = code;
        self
    }

    pub fn layout(mut self, name: impl Into<String>) -> Self {
        self.layout = Some(name.into());
        self
    }

    pub fn no_layout(mut self) -> Self {
        self.layout = Some(String::new());
        self
    }
}
