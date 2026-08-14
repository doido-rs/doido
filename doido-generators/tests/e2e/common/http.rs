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

/// POST JSON tolerating 4xx/5xx (ureq treats those as errors by default), so a
/// rejected sign-in (401) can be asserted without panicking. Returns status +
/// any `Set-Cookie` headers.
pub fn post_json_status_any(url: &str, body: Value) -> HttpResponse {
    let response = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .new_agent()
        .post(url)
        .send_json(body)
        .expect("POST request");
    HttpResponse {
        status: response.status().as_u16(),
        set_cookie: collect_set_cookie(response),
    }
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

/// GET returning the raw status, tolerating 4xx/5xx (ureq treats those as errors
/// by default). Used to assert a route is *absent* without panicking on the 404.
pub fn get_status_any(url: &str) -> u16 {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .new_agent()
        .get(url)
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

fn no_redirect_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .max_redirects(0)
        .build()
        .new_agent()
}

fn collect_set_cookie(response: ureq::http::Response<ureq::Body>) -> Vec<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok().map(str::to_string))
        .collect()
}

pub fn post_form(url: &str, fields: &[(&str, &str)]) -> u16 {
    post_form_with_response(url, fields).status
}

pub fn post_form_with_response(url: &str, fields: &[(&str, &str)]) -> HttpResponse {
    let owned: Vec<(String, String)> = fields
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let response = no_redirect_agent()
        .post(url)
        .send_form(owned)
        .expect("POST form");
    HttpResponse {
        status: response.status().as_u16(),
        set_cookie: collect_set_cookie(response),
    }
}

pub fn delete_with_response(url: &str) -> HttpResponse {
    let response = no_redirect_agent()
        .delete(url)
        .call()
        .expect("DELETE request");
    HttpResponse {
        status: response.status().as_u16(),
        set_cookie: collect_set_cookie(response),
    }
}

pub fn get_text(url: &str) -> String {
    ureq::get(url)
        .call()
        .expect("GET request")
        .into_body()
        .read_to_string()
        .expect("response body")
}

/// GET with a session `Cookie` header, returning the raw status (tolerates
/// 4xx/5xx). Used to hit a protected route as a signed-in user.
pub fn get_status_with_cookie(url: &str, cookie: &str) -> u16 {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .new_agent()
        .get(url)
        .header("Cookie", cookie)
        .call()
        .expect("GET request")
        .status()
        .as_u16()
}

/// Extract the `_doido_session=<value>` pair from a response's `Set-Cookie`
/// headers, ready to be echoed back as a `Cookie` request header.
pub fn session_cookie(set_cookie: &[String]) -> String {
    set_cookie
        .iter()
        .find(|c| c.contains("_doido_session"))
        .and_then(|c| c.split(';').next())
        .unwrap_or_default()
        .to_string()
}

/// POST a form while sending a session `Cookie` header (authenticated request).
pub fn post_form_with_cookie(url: &str, fields: &[(&str, &str)], cookie: &str) -> HttpResponse {
    let owned: Vec<(String, String)> = fields
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let response = no_redirect_agent()
        .post(url)
        .header("Cookie", cookie)
        .send_form(owned)
        .expect("POST form");
    HttpResponse {
        status: response.status().as_u16(),
        set_cookie: collect_set_cookie(response),
    }
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
