//! Field-type mapping: a scaffold with one column per supported type must map
//! DB column ⇄ Rust/sea_orm type ⇄ JSON correctly. A single "kitchen-sink" model
//! is created, then a value of each type is round-tripped through the JSON API
//! (create → index → show) and asserted by both value and JSON type. This is the
//! authoritative check that `templates/models/model.rs.template` +
//! `field.rs` + the scaffold controller template agree with the migration.

use crate::common::db;
use crate::common::http;
use crate::common::{AppHarness, BaseProfile};
use serde_json::{json, Value};

/// The UUID we POST; sea_orm/serde round-trips it as a lowercase hyphenated string.
const TOKEN: &str = "11111111-1111-1111-1111-111111111111";

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn field_types_round_trip_create_list_show() {
    let h = AppHarness::new("type_mapping", BaseProfile::Default);
    h.generate(&[
        "generate",
        "scaffold",
        "TypeSample",
        "title:string",
        "body:text",
        "count:integer",
        "big_count:bigint",
        "ratio:float",
        "precise:double",
        "price:decimal",
        "active:boolean",
        "published_at:datetime",
        "due_on:date",
        "meta:json",
        "token:uuid",
        "payload:binary",
        "owner:references",
        "--api",
    ]);

    h.run_with_db(
        |h| {
            let app = &h.app;
            db::assert_table_exists(app, "type_samples");
            for col in [
                "title",
                "body",
                "count",
                "big_count",
                "ratio",
                "precise",
                "price",
                "active",
                "published_at",
                "due_on",
                "meta",
                "token",
                "payload",
                "owner_id",
            ] {
                db::assert_column_exists(app, "type_samples", col);
            }
            // DB side of the mapping for the unambiguous numeric/bool columns; the
            // rich types (decimal/datetime/date/json/uuid/binary) are validated by
            // the HTTP round-trip below (their SQLite type names are version-fragile).
            assert!(db::column_type(app, "type_samples", "count").contains("int"));
            assert!(db::column_type(app, "type_samples", "big_count").contains("int"));
            assert!(db::column_type(app, "type_samples", "owner_id").contains("int"));
            assert!(db::column_type(app, "type_samples", "active").contains("bool"));
        },
        |app| {
            let base = &app.base_url;
            let created = http::post_json(
                &format!("{base}/type_samples"),
                json!({
                    "title": "hello",
                    "body": "world",
                    "count": 42,
                    "big_count": 9_000_000_001i64,
                    "ratio": 1.5,
                    "precise": 2.25,
                    "price": "9.99",
                    "active": true,
                    "published_at": "2020-01-02T03:04:05",
                    "due_on": "2020-01-02",
                    "meta": { "k": "v", "n": 1 },
                    "token": TOKEN,
                    "payload": [1, 2, 3],
                    "owner_id": 7
                }),
            );
            let id = created["id"].as_i64().expect("created id");

            // Index returns the new row.
            let index = http::get_json(&format!("{base}/type_samples"));
            assert!(
                index
                    .as_array()
                    .expect("index array")
                    .iter()
                    .any(|row| row["id"] == json!(id)),
                "index should contain the created row"
            );

            // Show re-reads from the database — the definitive round-trip proving
            // storage + model type + serde all agree per field type.
            let show = http::get_json(&format!("{base}/type_samples/{id}"));
            assert_types(&show);
        },
    );
}

/// Assert every field round-trips with the correct value AND JSON type.
fn assert_types(v: &Value) {
    // string / text → JSON string
    assert_eq!(v["title"].as_str(), Some("hello"), "string");
    assert_eq!(v["body"].as_str(), Some("world"), "text");

    // integer / bigint / references → JSON integer number
    assert!(
        v["count"].is_i64() && v["count"].as_i64() == Some(42),
        "integer"
    );
    assert!(
        v["big_count"].is_i64() && v["big_count"].as_i64() == Some(9_000_000_001),
        "bigint"
    );
    assert!(
        v["owner_id"].is_i64() && v["owner_id"].as_i64() == Some(7),
        "references (owner_id)"
    );

    // float / double → JSON float number
    assert!(
        v["ratio"].is_f64() && v["ratio"].as_f64() == Some(1.5),
        "float"
    );
    assert!(
        v["precise"].is_f64() && v["precise"].as_f64() == Some(2.25),
        "double"
    );

    // boolean → JSON bool
    assert_eq!(v["active"].as_bool(), Some(true), "boolean");

    // decimal → serde default renders rust_decimal as a JSON string; accept a
    // numeric representation too so the test is robust to the serde config.
    assert!(
        v["price"] == json!("9.99") || v["price"].as_f64() == Some(9.99),
        "decimal should round-trip to 9.99, got {}",
        v["price"]
    );

    // datetime / date → JSON string (chrono Naive*)
    assert_eq!(
        v["published_at"].as_str(),
        Some("2020-01-02T03:04:05"),
        "datetime"
    );
    assert_eq!(v["due_on"].as_str(), Some("2020-01-02"), "date");

    // json → nested JSON value preserved
    assert!(v["meta"].is_object(), "json is object");
    assert_eq!(v["meta"]["k"].as_str(), Some("v"), "json string member");
    assert_eq!(v["meta"]["n"].as_i64(), Some(1), "json number member");

    // uuid → JSON string (case-insensitive)
    assert_eq!(
        v["token"].as_str().map(str::to_lowercase),
        Some(TOKEN.to_string()),
        "uuid"
    );

    // binary → serde_json renders Vec<u8> as a JSON array of numbers
    assert_eq!(v["payload"], json!([1, 2, 3]), "binary");
}
