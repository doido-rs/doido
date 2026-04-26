use crate::generator::{GeneratedFile, Generator};
use doido_core::Result;

pub struct ProjectGenerator;

impl Generator for ProjectGenerator {
    fn name(&self) -> &str {
        "new"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("new generator requires a name argument")
        })?;

        let database = args
            .iter()
            .find(|a| a.starts_with("--database="))
            .and_then(|a| a.split_once('=').map(|(_, v)| v))
            .unwrap_or("sqlite");

        match database {
            "sqlite" | "postgres" | "mysql" => {}
            other => {
                return Err(doido_core::anyhow::anyhow!(
                    "Unknown database: {}. Use sqlite, postgres, or mysql.",
                    other
                ))
            }
        }

        let db_url = match database {
            "postgres" => format!("postgres://localhost/{name}_development"),
            "mysql" => format!("mysql://localhost/{name}_development"),
            _ => "sqlite://db/development.db".to_string(),
        };

        let sqlx_feature = match database {
            "postgres" => "postgres",
            "mysql" => "mysql",
            _ => "sqlite",
        };

        Ok(vec![
            GeneratedFile {
                path: format!("{name}/Cargo.toml"),
                content: format!(
                    "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ndoido = \"0.1\"\nsqlx = {{ version = \"0.8\", features = [\"runtime-tokio\", \"{sqlx_feature}\"] }}\ntokio = {{ version = \"1\", features = [\"full\"] }}\n"
                ),
            },
            GeneratedFile {
                path: format!("{name}/src/main.rs"),
                content: "fn main() {\n    println!(\"Hello from your Doido app!\");\n}\n".to_string(),
            },
            GeneratedFile {
                path: format!("{name}/config/application.toml"),
                content: format!("[app]\nname = \"{name}\"\n\n[database]\nurl = \"{db_url}\"\n"),
            },
            GeneratedFile {
                path: format!("{name}/config/routes.rs"),
                content: "use doido_router::routes;\n\nroutes! {}\n".to_string(),
            },
            GeneratedFile {
                path: format!("{name}/app/controllers/.gitkeep"),
                content: String::new(),
            },
            GeneratedFile {
                path: format!("{name}/app/models/.gitkeep"),
                content: String::new(),
            },
            GeneratedFile {
                path: format!("{name}/views/layouts/application.html.tera"),
                content: format!(
                    "<!DOCTYPE html>\n<html>\n<head><title>{name}</title></head>\n<body>{{% block content %}}{{% endblock %}}</body>\n</html>\n"
                ),
            },
            GeneratedFile {
                path: format!("{name}/db/migrations/.gitkeep"),
                content: String::new(),
            },
            GeneratedFile {
                path: format!("{name}/tests/integration_test.rs"),
                content: "#[test]\nfn test_app_starts() {\n    assert!(true);\n}\n".to_string(),
            },
            GeneratedFile {
                path: format!("{name}/.gitignore"),
                content: "/target\n.env\nconfig/master.key\nconfig/credentials.yml.enc\n*.db\n"
                    .to_string(),
            },
        ])
    }
}
