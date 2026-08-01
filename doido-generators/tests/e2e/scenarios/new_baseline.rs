//! Baseline `doido new` app serves the hello endpoint.

use crate::common::http;
use crate::common::{AppHarness, BaseProfile};

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn new_sqlite_app_serves_hello() {
    let h = AppHarness::new("new_baseline", BaseProfile::Default);
    h.run(|app| {
        let body = http::get_json(&format!("{}/", app.base_url));
        assert_eq!(body["message"], "Hello word!");
    });
}
