use doido_cache::{CacheConfig, MultiCacheConfig};

#[tokio::test]
async fn multi_store_config_builds_a_named_registry() {
    let mut cfg = MultiCacheConfig::default();
    cfg.stores.insert("primary".into(), CacheConfig::default());
    cfg.stores.insert(
        "sessions".into(),
        CacheConfig {
            namespace: Some("sess".into()),
            ..Default::default()
        },
    );

    let registry = cfg.build_registry().await.unwrap();
    assert!(registry.store("primary").is_some());
    assert!(registry.store("sessions").is_some());
    assert!(registry.store("missing").is_none());
}
