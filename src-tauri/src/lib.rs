use reqwest::{
    Client, Method, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct RequestHeader {
    key: String,
    value: String,
}

#[derive(Serialize)]
struct PostResponse {
    status: u16,
    body: String,
}

#[tauri::command]
async fn send_request(
    method: String,
    domain: String,
    endpoint: String,
    body: String,
    headers: Vec<RequestHeader>,
) -> Result<PostResponse, String> {
    let method = Method::from_bytes(method.as_bytes())
        .map_err(|error| format!("Invalid HTTP method: {error}"))?;
    let url = build_url(&domain, &endpoint)?;
    let headers = parse_headers(headers)?;

    let response = Client::new()
        .request(method, url)
        .headers(headers)
        .body(body)
        .send()
        .await
        .map_err(|error| format!("HTTP request failed: {error}"))?;

    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read response body: {error}"))?;

    Ok(PostResponse { status, body })
}

fn build_url(domain: &str, endpoint: &str) -> Result<Url, String> {
    let domain = domain.trim();
    if domain.is_empty() {
        return Err("Domain is required.".to_owned());
    }

    let base = Url::parse(domain).map_err(|error| format!("Invalid domain: {error}"))?;
    if !matches!(base.scheme(), "http" | "https") || !base.has_host() {
        return Err("Domain must be an absolute http or https URL.".to_owned());
    }

    base.join(endpoint.trim())
        .map_err(|error| format!("Invalid endpoint: {error}"))
}

fn parse_headers(headers: Vec<RequestHeader>) -> Result<HeaderMap, String> {
    let mut header_map = HeaderMap::new();
    for header in headers {
        let name = HeaderName::from_bytes(header.key.as_bytes())
            .map_err(|error| format!("Invalid header name: {error}"))?;
        let value = HeaderValue::from_str(&header.value)
            .map_err(|error| format!("Invalid value for header {name}: {error}"))?;
        header_map.insert(name, value);
    }

    Ok(header_map)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![send_request])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
