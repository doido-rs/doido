//! `doido config` — deploy-time config materialization (optional Tera render).

use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Render `config/<env>.yml` through Tera (`get_env`) for deploy pipelines.
    ///
    /// Committed config should stay static; run this in CI or an entrypoint when you
    /// still author templates with `{{ get_env(...) }}` in a gitignored source file.
    Render {
        /// Environment file to render (`config/<env>.yml`).
        #[arg(long, default_value = "production")]
        env: String,
        /// Read from this path instead of `config/<env>.yml`.
        #[arg(long)]
        input: Option<PathBuf>,
        /// Write rendered YAML here (default: stdout).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn run(action: ConfigCommand) -> Result<(), String> {
    match action {
        ConfigCommand::Render { env, input, output } => {
            render_config(&env, input.as_deref(), output.as_deref())
        }
    }
}

fn render_config(
    env: &str,
    input: Option<&std::path::Path>,
    output: Option<&std::path::Path>,
) -> Result<(), String> {
    match env {
        "development" | "test" | "production" => {}
        other => {
            return Err(format!(
                "unknown environment {other:?}; expected development, test, or production"
            ));
        }
    }
    let path = input
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("config/{env}.yml")));
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let rendered = doido_core::config::render(&raw).map_err(|e| e.to_string())?;
    match output {
        Some(out) => {
            if let Some(parent) = out.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
                }
            }
            std::fs::write(out, rendered)
                .map_err(|e| format!("failed to write {}: {e}", out.display()))?;
        }
        None => print!("{rendered}"),
    }
    Ok(())
}
