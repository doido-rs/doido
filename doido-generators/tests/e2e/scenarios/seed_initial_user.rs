//! Initial-user seed: an app generated with `doido new --auth` ships a
//! `db/seed` that inserts an admin user (admin@example.com / password), so a
//! fresh project has a login out of the box. This also guards that the seed
//! crate compiles `app/models/user.rs` (it depends on doido-auth + chrono only
//! under `--auth`).

use crate::common::{db, AppHarness, BaseProfile};

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn seed_creates_initial_user() {
    let h = AppHarness::new("seed_initial_user", BaseProfile::WithAuthApi);
    h.build();
    h.prepare_database();
    db::assert_table_exists(&h.app, "users");

    // The generated seed inserts the initial user.
    h.seed_database();
    db::assert_row_count(&h.app, "users", 1);

    // Idempotent: re-running the seed does not duplicate the user.
    h.seed_database();
    db::assert_row_count(&h.app, "users", 1);
}
