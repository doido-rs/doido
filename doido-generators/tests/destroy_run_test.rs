//! Isolated binary: `destroy::run` uses paths relative to cwd.

use doido_generators::commands::destroy::run;
use std::fs;

#[test]
fn destroy_removes_generated_controller_files() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    fs::create_dir_all("src/controllers").unwrap();
    fs::write("src/controllers/posts_controller.rs", "// generated").unwrap();

    let removed = run("controller", &["Posts"]).unwrap();
    assert!(removed.iter().any(|p| p.contains("posts_controller")));
    assert!(!dir
        .path()
        .join("src/controllers/posts_controller.rs")
        .exists());
}
