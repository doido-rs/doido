//! `doido new --cable` plus channel generator — real cable round-trip via app test.

use crate::common::{AppHarness, BaseProfile};
use std::process::Command;

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn new_cable_channel_broadcasts() {
    let h = AppHarness::new("new_cable", BaseProfile::WithCable);
    h.generate(&["generate", "channel", "ChatRoom"]);
    h.configure_server();
    h.build();
    h.prepare_database();

    let target = crate::common::workspace::app_cargo_target(&h.app);
    let status = Command::new(env!("CARGO"))
        .args(["test", "--bin", "blog", "chat", "--", "--nocapture"])
        .current_dir(&h.app)
        .env("CARGO_TARGET_DIR", &target)
        .status()
        .expect("cargo test chat");
    assert!(status.success(), "cable channel unit test failed");

    h.run(|app| {
        let body = crate::common::http::get_json(&format!("{}/", app.base_url));
        assert_eq!(body["message"], "Hello word!");
    });
}
