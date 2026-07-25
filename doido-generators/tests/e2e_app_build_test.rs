//! End-to-end proof that `doido new` plus **every** generator produces an app
//! that actually compiles — the framework's e2e definition-of-done.
//!
//! It drives the real `doido` CLI (so each generator reads/writes the on-disk
//! app and its module/route injection accumulates, exactly as a user would),
//! then builds the whole app workspace against the in-tree framework crates.
//!
//! It builds the whole framework, so it is `#[ignore]`d and kept OUT of the fast
//! `make verify` gate — run it with `make example` (or
//! `cargo test -p doido-generators -- --ignored`).

use assert_cmd::Command;
use std::path::Path;
use std::process::Command as StdCommand;

/// The `doido` CLI, run inside `dir`.
fn doido(dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("doido-generators").expect("doido-generators binary");
    cmd.current_dir(dir);
    cmd
}

/// A scratch dir for the scaffolded app, placed under the workspace `target/`
/// (real disk, gitignored) rather than `/tmp`, which is frequently a small
/// tmpfs that a full framework + app build overflows.
fn e2e_tempdir() -> tempfile::TempDir {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("target/e2e");
    std::fs::create_dir_all(&base).expect("create e2e base dir");
    tempfile::Builder::new()
        .prefix("doido-e2e-")
        .tempdir_in(&base)
        .expect("create tempdir")
}

#[test]
#[ignore = "slow: builds the whole framework; run via `make example`"]
fn every_generator_output_compiles() {
    let tmp = e2e_tempdir();

    // `doido new blog --database=sqlite --cable` (cable so channels are wired).
    doido(tmp.path())
        .args(["new", "blog", "--database=sqlite", "--cable"])
        .assert()
        .success();
    let app = tmp.path().join("blog");

    // Run one of each generator. Each reads/writes the on-disk app, so module and
    // route injection accumulates across invocations.
    let generators: [&[&str]; 8] = [
        &["generate", "scaffold", "Post", "title:string", "body:text"],
        &["generate", "model", "Comment", "body:text"],
        &["generate", "controller", "Pages"],
        &["generate", "job", "SendEmail"],
        &["generate", "mailer", "UserMailer"],
        &["generate", "channel", "ChatRoom"],
        &[
            "generate",
            "migration",
            "add_views_to_posts",
            "views:integer",
        ],
        &[
            "generate",
            "migration",
            "remove_views_from_posts",
            "views:integer",
        ],
    ];
    for args in generators {
        doido(&app).args(args).assert().success();
    }

    // Build the whole app workspace (app + db/migration) against the framework.
    let status = StdCommand::new(env!("CARGO"))
        .args(["build", "--workspace", "--manifest-path"])
        .arg(app.join("Cargo.toml"))
        .status()
        .expect("run cargo build");
    assert!(
        status.success(),
        "a generated app or its migrations failed to compile"
    );
}
