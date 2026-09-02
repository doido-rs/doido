use criterion::{black_box, criterion_group, criterion_main, Criterion};
use doido_cache::{MemoryStore, CacheStore};
use serde_json::json;
use std::sync::Arc;

fn cache_get_set(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = Arc::new(MemoryStore::new());

    rt.block_on(async {
        store
            .set("bench-key", json!({"items": (0..50).collect::<Vec<_>>()}), None)
            .await
            .unwrap();
    });

    c.bench_function("memory_cache_get", |b| {
        b.to_async(&rt).iter(|| async {
            let value = store.get(black_box("bench-key")).await.unwrap();
            black_box(value);
        });
    });

    c.bench_function("memory_cache_set", |b| {
        b.to_async(&rt).iter(|| async {
            store
                .set(
                    black_box("bench-set"),
                    black_box(json!({"n": 42})),
                    None,
                )
                .await
                .unwrap();
        });
    });
}

criterion_group!(benches, cache_get_set);
criterion_main!(benches);
