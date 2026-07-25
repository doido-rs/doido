use doido_generators::generators::locale::LocaleGenerator;
use doido_generators::Generator;

#[test]
fn locale_generator_writes_a_locale_file() {
    let files = LocaleGenerator.generate(&["fr"]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "config/locales/fr.yml");
    assert!(files[0].content.starts_with("fr:"));

    // defaults to en
    let default = LocaleGenerator.generate(&[]).unwrap();
    assert_eq!(default[0].path, "config/locales/en.yml");
}
