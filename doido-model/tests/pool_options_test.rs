use doido_model::pool::pool_options;
use doido_model::sea_orm::Database;
use std::time::Duration;

#[test]
fn pool_options_without_tuning_still_builds() {
    let opts = pool_options("sqlite::memory:", true, None, None);
    assert_eq!(opts.get_max_connections(), None);
    assert_eq!(opts.get_connect_timeout(), None);
}

#[test]
fn test_lock_serializes_access() {
    let _a = doido_model::pool::test_lock();
    // Second lock in same thread would deadlock; we only assert it returns.
}

#[test]
fn pool_options_apply_the_knobs() {
    let opts = pool_options("sqlite::memory:", false, Some(7), Some(3));
    assert_eq!(opts.get_max_connections(), Some(7));
    assert_eq!(opts.get_connect_timeout(), Some(Duration::from_secs(3)));
}

#[tokio::test]
async fn a_tuned_pool_connects() {
    let opts = pool_options("sqlite::memory:", false, Some(5), Some(2));
    let conn = Database::connect(opts).await.unwrap();
    conn.ping().await.unwrap();
}
