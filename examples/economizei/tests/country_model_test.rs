#[test]
fn country_model_defines_bank_association() {
    let country = include_str!("../app/models/_entities/countries.rs");
    assert!(country.contains("has_many = \"super::banks::Entity\""));
}

#[test]
fn bank_model_belongs_to_country() {
    let bank = include_str!("../app/models/_entities/banks.rs");
    assert!(bank.contains("country_id: i64"));
    assert!(bank.contains("belongs_to = \"super::countries::Entity\""));
}
