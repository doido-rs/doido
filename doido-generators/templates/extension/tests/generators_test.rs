//! Harness for `{doido_ext_name}` generators.

use {doido_ext_crate_ident}::generators::{
    register, ExtensionGenerator, ExtensionGeneratorRegistry, InstallGenerator,
};

struct TestRegistry {
    names: Vec<String>,
}

impl ExtensionGeneratorRegistry for TestRegistry {
    fn register_extension(&mut self, generator: Box<dyn ExtensionGenerator>) {
        self.names.push(generator.name().to_string());
    }
}

#[test]
fn register_exports_install_generator() {
    let mut reg = TestRegistry { names: Vec::new() };
    register(&mut reg);
    assert_eq!(reg.names, vec!["{doido_ext_snake}:install".to_string()]);
}

#[test]
fn install_emits_readme() {
    let files = InstallGenerator.generate(&[]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "README.{doido_ext_snake}.md");
    assert!(files[0].content.contains("{doido_ext_pascal}"));
}
