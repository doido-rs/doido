use crate::generator::{GeneratedFile, Generator};
use crate::generators::{register_module, to_pascal, to_snake};
use doido_core::Result;

/// Fallback `app/jobs/mod.rs` when the app doesn't have one on disk yet.
const JOBS_MOD_BASE: &str = include_str!("../../templates/new/app/jobs/mod.rs");
const JOBS_MOD_PATH: &str = "app/jobs/mod.rs";

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

        // Register the job's module in app/jobs/mod.rs, preserving any existing.
        let existing =
            std::fs::read_to_string(JOBS_MOD_PATH).unwrap_or_else(|_| JOBS_MOD_BASE.to_string());
        let jobs_mod = register_module(&existing, &format!("{snake}_job"), "@generated-jobs");

        Ok(vec![
            GeneratedFile {
                path: format!("app/jobs/{snake}_job.rs"),
                content,
            },
            GeneratedFile {
                path: JOBS_MOD_PATH.to_string(),
                content: jobs_mod,
            },
            GeneratedFile {
                path: format!("tests/{snake}_job_test.rs"),
                content: test,
            },
        ])
    }
}
