use crate::commands::write_files;
use crate::generator::Generator;
use crate::generators::extension::ExtensionGenerator;
use std::path::Path;
use std::process::Command;

pub fn run_extension(name: &str) {
    match ExtensionGenerator.generate(&[name]) {
        Ok(files) => {
            if let Err(e) = write_files(&files, Path::new(".")) {
                doido_core::tracing::error!("error writing files: {e}");
                std::process::exit(1);
            }
            let crate_name = files
                .first()
                .and_then(|f| f.path.split('/').next())
                .unwrap_or(name);
            let git_result = Command::new("git").args(["init", crate_name]).output();
            match git_result {
                Ok(output) if output.status.success() => {
                    doido_core::tracing::info!("init {crate_name}/.git");
                }
                _ => {
                    doido_core::tracing::warn!(
                        "git init failed. Run it manually: git init {crate_name}"
                    );
                }
            }
            doido_core::tracing::info!(
                "created '{crate_name}'. Next: cd {crate_name} && cargo test"
            );
        }
        Err(e) => {
            doido_core::tracing::error!("{e}");
            std::process::exit(1);
        }
    }
}
