#[test]
fn bank_statement_import_routes_are_present() {
    let routes = include_str!("../config/routes.rs");
    assert!(routes.contains("BankStatementImportsController"));
    assert!(routes.contains("bank_statement_imports"));
}

#[test]
fn import_service_modules_exist() {
    assert!(std::path::Path::new("app/services/imports/mod.rs").exists());
    assert!(std::path::Path::new("app/services/imports/nubank.rs").exists());
    assert!(std::path::Path::new("app/services/imports/c6.rs").exists());
    assert!(
        std::path::Path::new("app/models/bank_statement_import.rs").exists()
    );
}

#[test]
fn import_views_exist() {
    assert!(std::path::Path::new(
        "app/views/bank_statement_imports/index.html.tera"
    )
    .exists());
    assert!(std::path::Path::new(
        "app/views/bank_statement_imports/form.html.tera"
    )
    .exists());
}
