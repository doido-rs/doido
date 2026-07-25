use doido_cache::multi::{fetch_multi, read_multi, write_multi};
use doido_cache::MemoryStore;
use serde_json::json;

#[tokio::test]
async fn multi_read_write_and_fetch() {
    let store = MemoryStore::new();
    write_multi(&store, &[("a", json!(1)), ("b", json!(2))], None)
        .await
        .unwrap();

    let got = read_multi(&store, &["a", "b", "c"]).await.unwrap();
    assert_eq!(got.len(), 2, "missing key c omitted");
    assert_eq!(got["a"], json!(1));

    let fetched = fetch_multi(&store, &["a", "c"], None, |k| {
        json!(format!("computed:{k}"))
    })
    .await
    .unwrap();
    assert_eq!(fetched["a"], json!(1), "cached hit");
    assert_eq!(fetched["c"], json!("computed:c"), "computed miss");

    // c is now cached
    assert!(read_multi(&store, &["c"]).await.unwrap().contains_key("c"));
}
