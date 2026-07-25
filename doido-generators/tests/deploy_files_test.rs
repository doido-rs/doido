use doido_generators::{Generator, ProjectGenerator};

#[test]
fn new_app_includes_deploy_and_devcontainer_files() {
    let files = ProjectGenerator
        .generate(&["blog", "--database=sqlite"])
        .unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();

    assert!(paths.iter().any(|p| p.ends_with("Dockerfile")), "{paths:?}");
    assert!(paths.iter().any(|p| p.ends_with("config/deploy.yml")));
    assert!(paths.iter().any(|p| p.ends_with("devcontainer.json")));
}
