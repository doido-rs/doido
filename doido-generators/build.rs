//! Captures the absolute path of the doido workspace root and injects it into a
//! rustc env var so generated apps can depend on the local `doido-*` crates by
//! path (local development workflow) instead of crates.io releases.
//!
//! It also decides whether generated apps should use `path` or `version`
//! dependencies: when this crate is built inside the doido workspace (the
//! sibling `doido` crate is on disk) apps get `path` deps so they build against
//! the in-tree framework; once `doido-generators` is packaged/published the
//! siblings are gone, so apps get `version` deps that resolve from crates.io.

use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    // doido-generators lives at `<workspace>/doido-generators`, so its parent is
    // the workspace root that holds the sibling `doido-*` member crates.
    let workspace_root = manifest_dir.join("..");
    let workspace_root = workspace_root.canonicalize().unwrap_or(workspace_root);

    // The sibling `doido` crate is present only in a local workspace checkout,
    // not in a packaged/published tarball. Its presence is our signal for which
    // dependency style generated apps should use.
    let use_path_deps = workspace_root.join("doido").join("Cargo.toml").is_file();

    println!(
        "cargo:rustc-env=DOIDO_GENERATOR_TEMPLATE_WORKSPACE_PATH={}",
        workspace_root.display()
    );
    println!(
        "cargo:rustc-env=DOIDO_GENERATOR_USE_PATH_DEPS={}",
        if use_path_deps { "1" } else { "0" }
    );
    println!("cargo:rerun-if-changed=build.rs");
    // `include_dir!` embeds templates/ at compile time but doesn't track files
    // added or removed there; watch the tree so a rebuild re-embeds it.
    println!("cargo:rerun-if-changed=templates");
}
