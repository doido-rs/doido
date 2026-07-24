//! End-to-end proof that `doido new` scaffolds an app that actually compiles,
//! and that the migration generator's `doido_model::migration` output compiles.
//!
//! This is the framework's e2e definition-of-done: it generates a fresh app into
//! a tempdir (so nothing machine-specific is committed) plus one migration per
//! Rails pattern, and builds the whole app workspace against the in-tree
//! framework crates via the generator's path dependencies.
//!
//! It builds the whole framework, so it is `#[ignore]`d and kept OUT of the fast
//! `make verify` gate — run it with `make example` (or
//! `cargo test -p doido-generators -- --ignored`).

use doido_generators::generators::migration::MigrationGenerator;
use doido_generators::generators::migration_support::register_migration;
use doido_generators::generators::new::ProjectGenerator;
use doido_generators::Generator;
use std::path::Path;
use std::process::Command;

#[test]
#[ignore = "slow: builds the whole framework; run via `make example`"]
fn generated_app_and_migrations_compile() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let app = dir.path().join("blog");

    // `doido new blog --database=sqlite`, written straight to disk.
    let files = ProjectGenerator
        .generate(&["blog", "--database=sqlite"])
        .expect("generate the blog app");
    for f in &files {
        let path = dir.path().join(&f.path);
        std::fs::create_dir_all(path.parent().unwrap()).expect("create dirs");
        std::fs::write(&path, &f.content).expect("write file");
    }

    // Prove the migration generator's doido-model output compiles: one migration
    // per Rails pattern (create / add columns / remove columns), each registered
    // into the Migrator and built as part of the app workspace.
    let patterns: [&[&str]; 3] = [
        &["create_widgets", "name:string:unique", "note:text"],
        &["add_price_to_widgets", "price:integer"],
        &["remove_price_from_widgets", "price:integer"],
    ];
    let lib_path = app.join("db/migration/src/lib.rs");
    let mut lib = std::fs::read_to_string(&lib_path).expect("read migration lib.rs");
    for spec in patterns {
        for f in &MigrationGenerator
            .generate(spec)
            .expect("generate migration")
        {
            // Take the migration module file; assemble lib.rs ourselves.
            let is_migration = f.path.starts_with("db/migration/src/")
                && f.path.ends_with(".rs")
                && !f.path.ends_with("lib.rs");
            if is_migration {
                let module = Path::new(&f.path).file_stem().unwrap().to_str().unwrap();
                std::fs::write(app.join(&f.path), &f.content).expect("write migration");
                lib = register_migration(&lib, module);
            }
        }
    }
    std::fs::write(&lib_path, &lib).expect("write migration lib.rs");

    // Build the app and its migration crate against the in-tree framework.
    let status = Command::new(env!("CARGO"))
        .args(["build", "--workspace", "--manifest-path"])
        .arg(app.join("Cargo.toml"))
        .status()
        .expect("run cargo build");
    assert!(
        status.success(),
        "the generated app or its migrations failed to compile"
    );
}
