//! Captures the absolute path of the doido workspace root and injects it into a
//! rustc env var so generated apps can depend on the local `doido-*` crates by
//! path (local development workflow) instead of crates.io releases.

use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    // doido-generators lives at `<workspace>/doido-generators`, so its parent is
    // the workspace root that holds the sibling `doido-*` member crates.
    let workspace_root = manifest_dir.join("..");
    let workspace_root = workspace_root.canonicalize().unwrap_or(workspace_root);

    println!(
        "cargo:rustc-env=DOIDO_GENERATOR_TEMPLATE_WORKSPACE_PATH={}",
        workspace_root.display()
    );
    println!("cargo:rerun-if-changed=build.rs");
}
