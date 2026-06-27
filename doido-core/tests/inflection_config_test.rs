use doido_core::inflector::{config::InflectionConfig, Inflections};

#[test]
fn empty_yaml_parses_to_default_config() {
    let cfg = InflectionConfig::from_yaml("").unwrap();
    assert!(cfg.irregulars.is_empty());
    assert!(cfg.uncountables.is_empty());
}

#[test]
fn applies_irregulars_over_defaults() {
    let yaml = r#"
irregulars:
  - { singular: goose, plural: geese }
"#;
    let cfg = InflectionConfig::from_yaml(yaml).unwrap();
    let mut i = Inflections::default();
    cfg.apply(&mut i);

    assert_eq!(i.pluralize("goose"), "geese");
    assert_eq!(i.singularize("geese"), "goose");
    // Defaults still work.
    assert_eq!(i.pluralize("post"), "posts");
}

#[test]
fn applies_uncountables_and_acronyms() {
    let yaml = r#"
uncountables:
  - bitcoin
acronyms:
  - API
"#;
    let cfg = InflectionConfig::from_yaml(yaml).unwrap();
    let mut i = Inflections::default();
    cfg.apply(&mut i);

    assert_eq!(i.pluralize("bitcoin"), "bitcoin");
    assert_eq!(i.camelize("api_client"), "APIClient");
}

#[test]
fn applies_regex_rules() {
    let yaml = r#"
plurals:
  - { pattern: "(quiz)$", replacement: "${1}zes" }
singulars:
  - { pattern: "(quiz)zes$", replacement: "${1}" }
"#;
    let cfg = InflectionConfig::from_yaml(yaml).unwrap();
    let mut i = Inflections::default();
    cfg.apply(&mut i);

    assert_eq!(i.pluralize("quiz"), "quizzes");
    assert_eq!(i.singularize("quizzes"), "quiz");
}

#[test]
fn unknown_keys_are_rejected() {
    let err = InflectionConfig::from_yaml("bogus: true");
    assert!(err.is_err());
}

#[test]
fn load_missing_file_falls_back_to_defaults() {
    // A non-existent path is not an error; defaults are installed.
    let found =
        doido_core::load_inflections("config/__does_not_exist__.yaml").expect("missing file is ok");
    assert!(!found);
}
