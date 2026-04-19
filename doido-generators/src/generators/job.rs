use crate::generator::{GeneratedFile, Generator};
use crate::generators::to_snake;
use doido_core::Result;

pub struct JobGenerator;

impl Generator for JobGenerator {
    fn name(&self) -> &str {
        "job"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args
            .first()
            .copied()
            .ok_or_else(|| doido_core::anyhow::anyhow!("job generator requires a name argument"))?;
        let snake = to_snake(name);
        Ok(vec![GeneratedFile {
            path: format!("app/jobs/{}_job.rs", snake),
            content: format!(
                "use doido_jobs::job;\n\n#[job(max_retries = 3, queue = \"default\")]\nasync fn {snake}_job(payload: serde_json::Value) -> doido_core::Result<()> {{\n    // TODO: implement job\n    Ok(())\n}}\n",
            ),
        }])
    }
}
