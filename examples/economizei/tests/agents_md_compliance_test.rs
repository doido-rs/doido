#[test]
fn routes_module_is_present() {
    let routes = include_str!("../config/routes.rs");
    assert!(routes.contains("DashboardController::index"));
    assert!(routes.contains("ReportsController::index"));
    assert!(routes.contains("post!(\"/users/sign_out\""));
    assert!(routes.contains("MeController::company_users"));
    assert!(routes.contains("BankAccountsController"));
    assert!(routes.contains("auth_routes!("));
    assert!(routes.contains("auth::SessionsController"));
    assert!(routes.contains("collection: [export]"));
    assert!(!routes.contains("resources!(companies"));
    assert!(!routes.contains("resources!(memberships"));
    assert!(!routes.contains("scope!(\"/me\""));
    assert!(!routes.contains("/companies"));
    assert!(!routes.contains("/me/"));
    assert!(routes.contains("get!(\"/me\""));
    assert!(std::path::Path::new("app/services/listing.rs").exists());
    assert!(std::path::Path::new("app/views/banks/index.html.tera").exists());
}

#[test]
fn sidebar_navigation_matches_access_rules() {
    let sidebar = include_str!("../app/views/layouts/_sidebar.html.tera");
    assert!(sidebar.contains("/bank_accounts"));
    assert!(!sidebar.contains("/bank_accounts/new"));
    assert!(sidebar.contains("transactions_path"));
    assert!(sidebar.contains("nav.transactions"));
    assert!(sidebar.contains("categories_path"));
    assert!(sidebar.contains("nav.categories"));
    assert!(sidebar.contains("counterparties_path"));
    assert!(sidebar.contains("nav.counterparties"));
    assert!(sidebar.contains("/members"));
    assert!(!sidebar.contains("/me/"));
    assert!(!sidebar.contains("/companies"));
    assert!(sidebar.contains("/reports"));
    assert!(!sidebar.contains("/reports/health"));
    assert!(!sidebar.contains("/reports/spending_goals"));
}

#[test]
fn frontend_assets_are_configured() {
    let package = include_str!("../package.json");
    assert!(package.contains("css:build"));
    assert!(package.contains("tailwindcss"));

    let css = include_str!("../app/assets/stylesheets/application.css");
    assert!(css.contains("@import \"tailwindcss\""));
}

#[test]
fn paginator_partial_exists() {
    let paginator = include_str!("../app/views/layouts/_paginator.html.tera");
    assert!(paginator.contains("per_page"));
    assert!(paginator.contains("export_path"));
}

#[test]
fn sidebar_layout_exists() {
    let layout = include_str!("../app/views/layouts/application.html.tera");
    assert!(layout.contains("layouts/_sidebar.html.tera"));
    assert!(layout.contains("Ubuntu+Mono"));

    let sidebar = include_str!("../app/views/layouts/_sidebar.html.tera");
    assert!(sidebar.contains("<aside"));
}
