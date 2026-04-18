use doido_core::Result;

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

pub trait Generator {
    fn name(&self) -> &str;
    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>>;
}
