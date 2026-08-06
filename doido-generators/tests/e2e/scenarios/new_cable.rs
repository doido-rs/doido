//! `doido new --cable` plus channel generator — real cable round-trip via app test.

use crate::common::http;
use crate::common::{cli, AppHarness, BaseProfile};

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn new_cable_channel_broadcasts() {
    let h = AppHarness::new("new_cable", BaseProfile::WithCable);
    h.generate(&["generate", "channel", "ChatRoom"]);
    h.run_with_db(
        |h| {
            let status = cli::cargo_test(&h.app, &["--bin", "blog", "chat", "--", "--nocapture"]);
            assert!(status.success(), "cable channel unit test failed");
        },
        |app| {
            let body = http::get_json(&format!("{}/", app.base_url));
            assert_eq!(body["message"], "Hello, world!");
        },
    );
}
