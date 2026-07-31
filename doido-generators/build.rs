//! Watches generator templates so `include_dir!` re-embeds them when they change.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // `include_dir!` embeds templates/ at compile time but doesn't track files
    // added or removed there; watch the tree so a rebuild re-embeds it.
    println!("cargo:rerun-if-changed=templates");
}
