//! Pins `doido` / `doido-controller` versions from the sibling workspace crates into rustc env vars
//! so `Cargo.toml` stubs in `templates/app/` match the toolchain that compiled `doido-generators`.
//! Fallback: [`env!("CARGO_PKG_VERSION")`] of this crate (e.g. standalone `cargo package` tarball).

use std::fs;
use std::path::{Path, PathBuf};

fn read_workspace_package_version(workspace_root_manifest: &Path) -> Option<String> {
    let txt = fs::read_to_string(workspace_root_manifest).ok()?;
    let val: toml::Value = txt.parse().ok()?;
    val.get("workspace")?
        .get("package")?
        .get("version")?
        .as_str()
        .map(ToOwned::to_owned)
}

/// Resolves `[package].version`: explicit string vs `version.workspace = true`.
fn resolve_package_version(pkg: &toml::Table, workspace_fallback: Option<&str>) -> Option<String> {
    match pkg.get("version") {
        Some(toml::Value::String(v)) => Some(v.clone()),
        Some(toml::Value::Table(t)) => {
            if t.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
                workspace_fallback.map(str::to_owned)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn read_member_manifest_version(
    member_manifest: &Path,
    workspace_fallback: Option<&str>,
) -> Option<String> {
    let txt = fs::read_to_string(member_manifest).ok()?;
    let val: toml::Value = txt.parse().ok()?;
    let pkg = val.get("package")?.as_table()?;
    resolve_package_version(pkg, workspace_fallback)
}

fn resolve_pinned_version(member_manifest: &Path, workspace_root_manifest: &Path) -> String {
    let fallback = env!("CARGO_PKG_VERSION").to_owned();
    if !workspace_root_manifest.is_file() || !member_manifest.is_file() {
        return fallback;
    }
    let ws_ver = read_workspace_package_version(workspace_root_manifest);
    read_member_manifest_version(member_manifest, ws_ver.as_deref()).unwrap_or(fallback)
}

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo_root = manifest_dir.join("..");
    let workspace_manifest = repo_root.join("Cargo.toml");
    let doido_manifest = repo_root.join("doido/Cargo.toml");
    let controller_manifest = repo_root.join("doido-controller/Cargo.toml");

    let doido_ver = resolve_pinned_version(&doido_manifest, &workspace_manifest);
    let controller_ver = resolve_pinned_version(&controller_manifest, &workspace_manifest);

    println!("cargo:rustc-env=DOIDO_GENERATOR_TEMPLATE_DOIDO_VERSION={doido_ver}");
    println!("cargo:rustc-env=DOIDO_GENERATOR_TEMPLATE_DOIDO_CONTROLLER_VERSION={controller_ver}");

    println!("cargo:rerun-if-changed={}", workspace_manifest.display());
    println!("cargo:rerun-if-changed={}", doido_manifest.display());
    println!("cargo:rerun-if-changed={}", controller_manifest.display());
}
