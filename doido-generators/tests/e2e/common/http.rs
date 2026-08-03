//! HTTP helpers for real request/response interactions.

use serde_json::Value;

pub struct HttpResponse {
    pub status: u16,
    pub set_cookie: Vec<String>,
}

pub fn post_json_with_response(url: &str, body: Value) -> HttpResponse {
    let response = ureq::post(url).send_json(body).expect("POST request");
    let status = response.status().as_u16();
    let set_cookie = response
        .headers()
        .get("set-cookie")
        .map(|v| v.to_str().unwrap_or_default().to_string())
        .into_iter()
        .collect();
    HttpResponse { status, set_cookie }
}

pub fn get_json(url: &str) -> Value {
    ureq::get(url)
        .call()
        .expect("GET request")
        .into_body()
        .read_json()
        .expect("json body")
}

pub fn post_json(url: &str, body: Value) -> Value {
    ureq::post(url)
        .send_json(body)
        .expect("POST request")
        .into_body()
        .read_json()
        .expect("json body")
}

pub fn patch_json(url: &str, body: Value) -> Value {
    ureq::patch(url)
        .send_json(body)
        .expect("PATCH request")
        .into_body()
        .read_json()
        .expect("json body")
}

pub fn get_status(url: &str) -> u16 {
    ureq::get(url)
        .call()
        .expect("GET request")
        .status()
        .as_u16()
}

pub fn delete_status(url: &str) -> u16 {
    ureq::delete(url)
        .call()
        .expect("DELETE request")
        .status()
        .as_u16()
}

pub fn post_form(url: &str, fields: &[(&str, &str)]) -> u16 {
    let owned: Vec<(String, String)> = fields
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    ureq::post(url)
        .send_form(owned)
        .expect("POST form")
        .status()
        .as_u16()
}

pub fn get_text(url: &str) -> String {
    ureq::get(url)
        .call()
        .expect("GET request")
        .into_body()
        .read_to_string()
        .expect("response body")
}

pub fn api_crud_cycle(base: &str, collection: &str, create_body: Value, update_body: Value) {
    let created = post_json(&format!("{base}/{collection}"), create_body);
    let id = created["id"].as_i64().expect("created id");

    let index = get_json(&format!("{base}/{collection}"));
    assert!(index.as_array().unwrap().iter().any(|row| row["id"] == id));

    assert_eq!(get_status(&format!("{base}/{collection}/{id}")), 200);

    let updated = patch_json(&format!("{base}/{collection}/{id}"), update_body.clone());
    if let Some(expected) = update_body.as_object() {
        for (key, value) in expected {
            assert_eq!(&updated[key], value, "PATCH should persist `{key}`");
        }
    }

    assert_eq!(delete_status(&format!("{base}/{collection}/{id}")), 204);
}
