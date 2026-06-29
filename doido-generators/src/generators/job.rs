use crate::generator::{GeneratedFile, Generator};
use crate::generators::{to_pascal, to_snake};
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
        let pascal = to_pascal(name);
        let content = crate::templates::get("job/job.rs.template")
            .replace("{pascal}", &pascal)
            .replace("{snake}", &snake);
        let test = crate::templates::get("job/job_test.rs.template")
            .replace("{pascal}", &pascal)
            .replace("{snake}", &snake);
        Ok(vec![
            GeneratedFile {
                path: format!("app/jobs/{snake}_job.rs"),
                content,
            },
            GeneratedFile {
                path: format!("tests/{snake}_job_test.rs"),
                content: test,
            },
        ])
    }
}
