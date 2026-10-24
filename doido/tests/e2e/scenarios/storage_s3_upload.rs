//! S3 direct upload over HTTP against Floci (LocalStack-compatible S3 emulator).
//!
//! Self-skips when the S3 emulator is not reachable (local dev without Docker).

use crate::common::http;
use crate::common::{AppHarness, BaseProfile};
use serde_json::json;
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

fn s3_emulator_available() -> bool {
    let addr: SocketAddr = "127.0.0.1:4566".parse().expect("addr");
    TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok()
}

fn enable_storage_s3(app: &Path) {
    let path = app.join("Cargo.toml");
    let cargo = fs::read_to_string(&path).unwrap();
    let patched = cargo
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("doido-controller = ") || trimmed.starts_with("doido-model = ") {
                return line
                    .replace(", \"storage-s3\"", "")
                    .replace("\"storage-s3\", ", "");
            }
            if trimmed.starts_with("doido = ") {
                if line.contains("storage-s3") {
                    return line.to_string();
                }
                if line.contains("features = [\"sqlite\"]") {
                    return line.replace(
                        "features = [\"sqlite\"]",
                        "features = [\"sqlite\", \"storage-s3\"]",
                    );
                }
                if line.contains("features = [\"sqlite\", \"auth\"]") {
                    return line.replace(
                        "features = [\"sqlite\", \"auth\"]",
                        "features = [\"sqlite\", \"auth\", \"storage-s3\"]",
                    );
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        patched.contains("storage-s3"),
        "could not patch Cargo.toml to enable storage-s3:\n{cargo}"
    );
    fs::write(path, patched).unwrap();
}

fn configure_s3_storage(app: &Path) {
    let path = app.join("config/development.yml");
    let mut yaml = fs::read_to_string(&path).unwrap();
    if let Some(start) = yaml.find("storage:") {
        yaml.truncate(start);
        yaml = yaml.trim_end().to_string();
        yaml.push('\n');
    }
    yaml.push_str(
        r#"storage:
  driver: amazon
  drivers:
    local:
      type: disk
      root: storage
    test:
      type: memory
    amazon:
      type: s3
      bucket: doido-test
      region: us-east-1
      endpoint: http://127.0.0.1:4566
      access_key_id: test
      secret_access_key: test
"#,
    );
    fs::write(path, yaml).unwrap();
}

fn put_bytes(url: &str, body: &[u8]) -> u16 {
    ureq::put(url)
        .send(body)
        .expect("PUT presigned upload")
        .status()
        .as_u16()
}

#[test]
#[ignore = "slow: release e2e — run via `make release-e2e`"]
fn storage_s3_direct_upload_and_proxy() {
    if !s3_emulator_available() {
        eprintln!("Floci/S3 emulator (127.0.0.1:4566) not reachable; skipping storage S3 e2e");
        return;
    }

    let h = AppHarness::new("storage_s3_upload", BaseProfile::Default);
    h.generate(&["generate", "storage:install"]);
    enable_storage_s3(&h.app);
    configure_s3_storage(&h.app);

    h.run_with_db(
        |h| {
            crate::common::db::assert_table_exists(&h.app, "storage_blobs");
        },
        |app| {
            let create = http::post_json(
                &format!("{}/doido/storage/direct_uploads", app.base_url),
                json!({
                    "filename": "upload.txt",
                    "content_type": "text/plain"
                }),
            );
            let upload_url = create["direct_upload"]["url"]
                .as_str()
                .expect("presigned upload url");
            assert!(
                upload_url.contains("4566") || upload_url.contains("localstack"),
                "expected S3/LocalStack presigned URL, got {upload_url}"
            );
            assert!(
                !upload_url.contains("/disk/"),
                "S3 driver should not fall back to disk PUT route"
            );

            let put_status = put_bytes(upload_url, b"uploaded via s3 e2e");
            assert!(
                put_status == 200 || put_status == 204,
                "presigned PUT failed with status {put_status}"
            );

            let signed_id = create["signed_id"].as_str().expect("signed_id");
            let proxy = format!(
                "{}/doido/storage/blobs/proxy/{signed_id}/upload.txt",
                app.base_url
            );
            let body = http::get_text(&proxy);
            assert_eq!(body, "uploaded via s3 e2e");
        },
    );
}
