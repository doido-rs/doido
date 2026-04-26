# doido generators: `new` and `generate` commands — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `doido new <app-name> [--database=sqlite|postgres|mysql]` to scaffold new Doido projects, and make `doido generate` actually write files to disk.

**Architecture:** `ProjectGenerator` implements the existing `Generator` trait and is registered in `default_registry()` under `"new"`. `doido-cli` adds `Commands::New` with interactive database prompting, dispatches to the registry, writes files via a shared `write_files` helper, and runs `git init`. Both `new` and `generate` use the same `write_files` helper.

**Tech Stack:** Rust, `doido-generators` (`Generator` trait, `GeneratorRegistry`, `GeneratedFile`), `doido-cli` (`clap`), `std::fs`, `std::process::Command`, `std::io` (interactive prompt), `tempfile` (tests)

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `doido-generators/src/generators/new.rs` | `ProjectGenerator` — generates all project scaffold files |
| Modify | `doido-generators/src/generators/mod.rs` | add `pub mod new` |
| Modify | `doido-generators/src/lib.rs` | re-export `ProjectGenerator`, register in `default_registry()` |
| Create | `doido-generators/tests/new_generator_test.rs` | unit tests for `ProjectGenerator` |
| Modify | `doido-cli/Cargo.toml` | add `tempfile = "3"` to dev-dependencies |
| Modify | `doido-cli/src/commands/mod.rs` | add `pub mod new`, `pub(crate) fn write_files`, unit tests |
| Create | `doido-cli/src/commands/new.rs` | `run_new` — prompt, dispatch, write files, git init |
| Modify | `doido-cli/src/lib.rs` | add `Commands::New` variant + dispatch arm |
| Modify | `doido-cli/src/commands/generate.rs` | call `write_files` instead of just printing |

---

### Task 1: Write failing tests for `ProjectGenerator`

**Files:**
- Create: `doido-generators/tests/new_generator_test.rs`

- [ ] **Step 1: Create the test file**

Create `doido-generators/tests/new_generator_test.rs`:

```rust
use doido_generators::generators::new::ProjectGenerator;
use doido_generators::{default_registry, Generator};

#[test]
fn test_new_generates_all_expected_files() {
    let files = ProjectGenerator.generate(&["my-app", "--database=sqlite"]).unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"my-app/Cargo.toml"));
    assert!(paths.contains(&"my-app/src/main.rs"));
    assert!(paths.contains(&"my-app/config/application.toml"));
    assert!(paths.contains(&"my-app/config/routes.rs"));
    assert!(paths.contains(&"my-app/app/controllers/.gitkeep"));
    assert!(paths.contains(&"my-app/app/models/.gitkeep"));
    assert!(paths.contains(&"my-app/views/layouts/application.html.tera"));
    assert!(paths.contains(&"my-app/db/migrations/.gitkeep"));
    assert!(paths.contains(&"my-app/tests/integration_test.rs"));
    assert!(paths.contains(&"my-app/.gitignore"));
}

#[test]
fn test_new_sqlite_cargo_toml_has_sqlite_feature() {
    let files = ProjectGenerator.generate(&["my-app", "--database=sqlite"]).unwrap();
    let cargo_toml = files.iter().find(|f| f.path == "my-app/Cargo.toml").unwrap();
    assert!(cargo_toml.content.contains("my-app"));
    assert!(cargo_toml.content.contains("sqlite"));
}

#[test]
fn test_new_postgres_sets_correct_database_url() {
    let files = ProjectGenerator.generate(&["blog", "--database=postgres"]).unwrap();
    let app_config = files.iter().find(|f| f.path == "blog/config/application.toml").unwrap();
    assert!(app_config.content.contains("postgres://localhost/blog_development"));
}

#[test]
fn test_new_mysql_sets_correct_database_url() {
    let files = ProjectGenerator.generate(&["store", "--database=mysql"]).unwrap();
    let app_config = files.iter().find(|f| f.path == "store/config/application.toml").unwrap();
    assert!(app_config.content.contains("mysql://localhost/store_development"));
}

#[test]
fn test_new_sqlite_default_when_no_database_flag() {
    let files = ProjectGenerator.generate(&["my-app"]).unwrap();
    let app_config = files.iter().find(|f| f.path == "my-app/config/application.toml").unwrap();
    assert!(app_config.content.contains("sqlite://db/development.db"));
}

#[test]
fn test_new_integration_test_file_has_passing_stub() {
    let files = ProjectGenerator.generate(&["my-app", "--database=sqlite"]).unwrap();
    let test_file = files.iter().find(|f| f.path == "my-app/tests/integration_test.rs").unwrap();
    assert!(test_file.content.contains("#[test]"));
    assert!(test_file.content.contains("assert!(true)"));
}

#[test]
fn test_new_output_is_deterministic() {
    let files1 = ProjectGenerator.generate(&["app1", "--database=sqlite"]).unwrap();
    let files2 = ProjectGenerator.generate(&["app1", "--database=sqlite"]).unwrap();
    let paths1: Vec<&str> = files1.iter().map(|f| f.path.as_str()).collect();
    let paths2: Vec<&str> = files2.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths1, paths2);
    assert_eq!(files1[0].content, files2[0].content);
}

#[test]
fn test_new_requires_name_argument() {
    let result = ProjectGenerator.generate(&[]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("name"));
}

#[test]
fn test_new_rejects_unknown_database() {
    let result = ProjectGenerator.generate(&["my-app", "--database=oracle"]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("oracle"));
}

#[test]
fn test_new_registered_in_default_registry() {
    let registry = default_registry();
    let files = registry.run("new", &["my-app", "--database=sqlite"]).unwrap();
    assert!(!files.is_empty());
}
```

- [ ] **Step 2: Run to confirm they fail**

```bash
cd /home/fabiano/projetos/doido/.worktrees/generators-new-generate
cargo test -p doido-generators --test new_generator_test 2>&1 | head -20
```

Expected: compilation error — `generators::new` module does not exist.

---

### Task 2: Implement `ProjectGenerator`

**Files:**
- Create: `doido-generators/src/generators/new.rs`
- Modify: `doido-generators/src/generators/mod.rs`

- [ ] **Step 1: Create `doido-generators/src/generators/new.rs`**

```rust
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
```

- [ ] **Step 2: Add `pub mod new` to `doido-generators/src/generators/mod.rs`**

The file currently ends with the existing module declarations. Append:

```rust
pub mod new;
```

- [ ] **Step 3: Run the failing tests (all except `test_new_registered_in_default_registry`)**

```bash
cd /home/fabiano/projetos/doido/.worktrees/generators-new-generate
cargo test -p doido-generators --test new_generator_test 2>&1
```

Expected: 9 of 10 pass; `test_new_registered_in_default_registry` fails with "generator 'new' not found".

---

### Task 3: Register `ProjectGenerator` in `default_registry()` and re-export it

**Files:**
- Modify: `doido-generators/src/lib.rs`

- [ ] **Step 1: Update `doido-generators/src/lib.rs`**

The current `lib.rs` has a `pub use generators::{ ... }` line and a `default_registry()` function. Make two changes:

1. Add to the `pub use generators::` block:
```rust
pub use generators::new::ProjectGenerator;
```

2. Add inside `default_registry()` after the last `reg.register(...)`:
```rust
reg.register(Box::new(ProjectGenerator));
```

- [ ] **Step 2: Run all generator tests**

```bash
cd /home/fabiano/projetos/doido/.worktrees/generators-new-generate
cargo test -p doido-generators 2>&1
```

Expected: all tests pass (including `test_new_registered_in_default_registry`).

- [ ] **Step 3: Commit**

```bash
cd /home/fabiano/projetos/doido/.worktrees/generators-new-generate
git add doido-generators/src/generators/new.rs \
        doido-generators/src/generators/mod.rs \
        doido-generators/src/lib.rs \
        doido-generators/tests/new_generator_test.rs
git commit -m "feat(generators): add ProjectGenerator for doido new command"
```

---

### Task 4: Add `write_files` helper and `tempfile` dev-dependency to `doido-cli`

**Files:**
- Modify: `doido-cli/Cargo.toml`
- Modify: `doido-cli/src/commands/mod.rs`

- [ ] **Step 1: Add `tempfile` dev-dependency to `doido-cli/Cargo.toml`**

In the `[dev-dependencies]` section, add:
```toml
tempfile = "3"
```

- [ ] **Step 2: Update `doido-cli/src/commands/mod.rs`**

Replace the entire file with:

```rust
pub mod console;
pub mod credentials;
pub mod db;
pub mod generate;
pub mod jobs;
pub mod server;
pub mod worker;

use doido_generators::GeneratedFile;
use std::fs;
use std::path::Path;

pub(crate) fn write_files(files: &[GeneratedFile], root: &Path) -> anyhow::Result<()> {
    for file in files {
        let dest = root.join(&file.path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, &file.content)?;
        println!("  create  {}", file.path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_write_files_creates_files_on_disk() {
        let dir = tempdir().unwrap();
        let files = vec![
            GeneratedFile {
                path: "src/main.rs".to_string(),
                content: "fn main() {}".to_string(),
            },
            GeneratedFile {
                path: "config/app.toml".to_string(),
                content: "[app]".to_string(),
            },
        ];
        write_files(&files, dir.path()).unwrap();
        assert!(dir.path().join("src/main.rs").exists());
        assert!(dir.path().join("config/app.toml").exists());
        assert_eq!(
            fs::read_to_string(dir.path().join("src/main.rs")).unwrap(),
            "fn main() {}"
        );
    }

    #[test]
    fn test_write_files_creates_nested_parent_directories() {
        let dir = tempdir().unwrap();
        let files = vec![GeneratedFile {
            path: "a/b/c/deep.rs".to_string(),
            content: "// deep".to_string(),
        }];
        write_files(&files, dir.path()).unwrap();
        assert!(dir.path().join("a/b/c/deep.rs").exists());
    }

    #[test]
    fn test_write_files_empty_content_creates_file() {
        let dir = tempdir().unwrap();
        let files = vec![GeneratedFile {
            path: "app/models/.gitkeep".to_string(),
            content: String::new(),
        }];
        write_files(&files, dir.path()).unwrap();
        assert!(dir.path().join("app/models/.gitkeep").exists());
    }
}
```

- [ ] **Step 3: Run `doido-cli` unit tests**

```bash
cd /home/fabiano/projetos/doido/.worktrees/generators-new-generate
cargo test -p doido-cli 2>&1
```

Expected: 3 new `write_files` tests pass; no regressions.

---

### Task 5: Implement `commands/new.rs`

**Files:**
- Modify: `doido-cli/src/commands/mod.rs`
- Create: `doido-cli/src/commands/new.rs`

- [ ] **Step 1: Add `pub mod new;` to `doido-cli/src/commands/mod.rs`**

Insert after `pub mod jobs;`:

```rust
pub mod new;
```

- [ ] **Step 2: Create `doido-cli/src/commands/new.rs`**

```rust
use crate::commands::write_files;
use doido_generators::default_registry;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn prompt_database() -> String {
    print!("Which database? [sqlite/postgres/mysql] (default: sqlite): ");
    io::stdout().flush().expect("failed to flush stdout");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("failed to read input");
    let trimmed = input.trim().to_lowercase();
    if trimmed.is_empty() {
        "sqlite".to_string()
    } else {
        trimmed
    }
}

pub fn run_new(name: &str, database: Option<&str>) {
    let db = match database {
        Some(d) => d.to_string(),
        None => prompt_database(),
    };

    match db.as_str() {
        "sqlite" | "postgres" | "mysql" => {}
        other => {
            eprintln!(
                "Error: Unknown database '{}'. Use sqlite, postgres, or mysql.",
                other
            );
            std::process::exit(1);
        }
    }

    let registry = default_registry();
    let db_arg = format!("--database={db}");
    match registry.run("new", &[name, &db_arg]) {
        Ok(files) => {
            let root = Path::new(".");
            if let Err(e) = write_files(&files, root) {
                eprintln!("Error writing files: {e}");
                std::process::exit(1);
            }
            let git_result = Command::new("git").args(["init", name]).output();
            match git_result {
                Ok(output) if output.status.success() => {
                    println!("      init  {name}/.git");
                }
                _ => eprintln!("Warning: git init failed. Run it manually: git init {name}"),
            }
            println!("\nCreated '{name}'. Next: cd {name} && cargo build");
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd /home/fabiano/projetos/doido/.worktrees/generators-new-generate
cargo build -p doido-cli 2>&1
```

Expected: compiles without errors. (Note: `pub mod new` is already added in Task 4 `commands/mod.rs`.)

---

### Task 6: Add `Commands::New` to `doido-cli/src/lib.rs` and update `run_generate`

**Files:**
- Modify: `doido-cli/src/lib.rs`
- Modify: `doido-cli/src/commands/generate.rs`

- [ ] **Step 1: Update `doido-cli/src/commands/generate.rs` to write files**

Replace the entire file with:

```rust
use crate::commands::write_files;
use doido_generators::default_registry;
use std::path::Path;

pub fn run_generate(generator: &str, args: &[&str]) {
    let registry = default_registry();
    match registry.run(generator, args) {
        Ok(files) => {
            if files.is_empty() {
                println!("  (no files generated)");
                return;
            }
            if let Err(e) = write_files(&files, Path::new(".")) {
                eprintln!("Error writing files: {e}");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
```

- [ ] **Step 2: Add `Commands::New` variant and dispatch to `doido-cli/src/lib.rs`**

In the `Commands` enum, add after `Commands::Generate { ... }`:

```rust
/// Create a new Doido application
New {
    /// Application name
    name: String,
    /// Database backend: sqlite, postgres, or mysql (prompted if omitted)
    #[arg(long)]
    database: Option<String>,
},
```

In the `match cli.command { ... }` block, add:

```rust
Commands::New { name, database } => {
    commands::new::run_new(&name, database.as_deref());
}
```

- [ ] **Step 3: Run all tests**

```bash
cd /home/fabiano/projetos/doido/.worktrees/generators-new-generate
cargo test 2>&1
```

Expected: all tests pass.

- [ ] **Step 4: Smoke-test the CLI**

```bash
cd /home/fabiano/projetos/doido/.worktrees/generators-new-generate
cargo run -p doido -- new test-app --database=sqlite 2>&1
ls test-app/
rm -rf test-app/
```

Expected output includes lines like:
```
  create  test-app/Cargo.toml
  create  test-app/src/main.rs
  ...
      init  test-app/.git
Created 'test-app'. Next: cd test-app && cargo build
```

- [ ] **Step 5: Commit**

```bash
cd /home/fabiano/projetos/doido/.worktrees/generators-new-generate
git add doido-cli/Cargo.toml \
        doido-cli/src/commands/mod.rs \
        doido-cli/src/commands/new.rs \
        doido-cli/src/commands/generate.rs \
        doido-cli/src/lib.rs
git commit -m "feat(cli): add doido new command and wire generate to write files"
```

---

### Task 7: CLI integration tests

**Files:**
- Modify: `doido-cli/tests/` (existing integration test location)

- [ ] **Step 1: Write integration tests using `assert_cmd`**

Create `doido-cli/tests/cli_new_test.rs`:

```rust
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_doido_new_creates_project_files() {
    let dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido").unwrap();
    cmd.current_dir(dir.path())
        .args(["new", "my-app", "--database=sqlite"])
        .assert()
        .success();

    assert!(dir.path().join("my-app/Cargo.toml").exists());
    assert!(dir.path().join("my-app/src/main.rs").exists());
    assert!(dir.path().join("my-app/config/application.toml").exists());
    assert!(dir.path().join("my-app/config/routes.rs").exists());
    assert!(dir.path().join("my-app/tests/integration_test.rs").exists());
    assert!(dir.path().join("my-app/.gitignore").exists());
}

#[test]
fn test_doido_new_cargo_toml_has_correct_app_name() {
    let dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido").unwrap();
    cmd.current_dir(dir.path())
        .args(["new", "blog-app", "--database=postgres"])
        .assert()
        .success();

    let cargo_toml = fs::read_to_string(dir.path().join("blog-app/Cargo.toml")).unwrap();
    assert!(cargo_toml.contains("blog-app"));
    assert!(cargo_toml.contains("postgres"));
}

#[test]
fn test_doido_new_creates_git_repository() {
    let dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido").unwrap();
    cmd.current_dir(dir.path())
        .args(["new", "my-app", "--database=sqlite"])
        .assert()
        .success();

    assert!(dir.path().join("my-app/.git").exists());
}

#[test]
fn test_doido_generate_model_writes_file() {
    let dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("doido").unwrap();
    cmd.current_dir(dir.path())
        .args(["generate", "model", "User"])
        .assert()
        .success();

    assert!(dir.path().join("app/models/user.rs").exists());
}
```

- [ ] **Step 2: Run the integration tests**

```bash
cd /home/fabiano/projetos/doido/.worktrees/generators-new-generate
cargo test -p doido-cli --test cli_new_test 2>&1
```

Expected: all 4 tests pass.

- [ ] **Step 3: Run the full test suite**

```bash
cd /home/fabiano/projetos/doido/.worktrees/generators-new-generate
cargo test 2>&1 | tail -20
```

Expected: all tests pass, no failures.

- [ ] **Step 4: Final commit**

```bash
cd /home/fabiano/projetos/doido/.worktrees/generators-new-generate
git add doido-cli/tests/cli_new_test.rs
git commit -m "test(cli): add integration tests for doido new and generate commands"
```
