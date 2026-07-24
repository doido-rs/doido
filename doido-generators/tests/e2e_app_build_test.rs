//! End-to-end proof that `doido new` scaffolds an app that actually compiles.
//!
//! This is the framework's e2e definition-of-done: it generates a fresh app into
//! a tempdir (so nothing machine-specific is committed) and builds it against the
//! in-tree framework crates via the generator's path dependencies.
//!
//! It builds the whole framework, so it is `#[ignore]`d and kept OUT of the fast
//! `make verify` gate — run it with `make example` (or
//! `cargo test -p doido-generators -- --ignored`).

use doido_generators::generators::new::ProjectGenerator;
use doido_generators::Generator;
use std::process::Command;

#[test]
#[ignore = "slow: builds the whole framework; run via `make example`"]
fn generated_app_compiles() {
    let dir = tempfile::tempdir().expect("create tempdir");

    // `doido new blog --database=sqlite`, written straight to disk.
    let files = ProjectGenerator
        .generate(&["blog", "--database=sqlite"])
        .expect("generate the blog app");
    for f in &files {
        let path = dir.path().join(&f.path);
        std::fs::create_dir_all(path.parent().unwrap()).expect("create dirs");
        std::fs::write(&path, &f.content).expect("write file");
    }

    // The generated Cargo.toml points its doido-* path deps at this workspace,
    // so the app builds against the in-tree framework.
    let manifest = dir.path().join("blog/Cargo.toml");
    let status = Command::new(env!("CARGO"))
        .args(["build", "--manifest-path"])
        .arg(&manifest)
        .status()
        .expect("run cargo build");

    assert!(status.success(), "the generated blog app failed to compile");
}
