use crate::commands::write_files;
use crate::default_registry;
use std::path::Path;

pub fn run_generate(generator: &str, args: &[&str]) {
    let registry = default_registry();
    match registry.run(generator, args) {
        Ok(files) => {
            if files.is_empty() {
                doido_core::tracing::info!("no files generated");
                return;
            }
            if let Err(e) = write_files(&files, Path::new(".")) {
                doido_core::tracing::error!("error writing files: {e}");
                std::process::exit(1);
            }
        }
        Err(e) => {
            doido_core::tracing::error!("{e}");
            std::process::exit(1);
        }
    }
}
