#[test]
fn bcb_import_service_is_wired_to_job() {
    let job = include_str!("../app/jobs/import_authorized_banks_job.rs");
    assert!(job.contains("bcb_import::import_authorized_banks"));
    assert!(job.contains("#[job"));
}

#[test]
fn boot_skips_test_environment() {
    let boot = include_str!("../app/boot.rs");
    assert!(boot.contains("Environment::Test"));
    assert!(boot.contains("is_server_command"));
}
