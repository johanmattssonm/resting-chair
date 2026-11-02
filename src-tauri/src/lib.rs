use std::{fs, io, path::PathBuf};

use reqwest::{
    Client, Method, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Clone, Deserialize, Serialize)]
struct RequestHeader {
    key: String,
    value: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredRequest {
    id: String,
    name: String,
    method: String,
    domain: String,
    endpoint: String,
    body: String,
    headers: Vec<RequestHeader>,
}

const REQUESTS_FILE_NAME: &str = "requests.json";

#[derive(Serialize)]
struct PostResponse {
    status: u16,
    body: String,
}

fn requests_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not find the application data directory: {error}"))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|error| format!("Could not create the application data directory: {error}"))?;

    Ok(app_data_dir.join(REQUESTS_FILE_NAME))
}

fn read_requests(app: &AppHandle) -> Result<Vec<StoredRequest>, String> {
    let path = requests_file_path(app)?;
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not read saved requests: {error}")),
    };

    serde_json::from_str(&contents)
        .map_err(|error| format!("Saved requests file is invalid JSON: {error}"))
}

fn write_requests(app: &AppHandle, requests: &[StoredRequest]) -> Result<(), String> {
    let contents = serde_json::to_string_pretty(requests)
        .map_err(|error| format!("Could not serialize saved requests: {error}"))?;
    fs::write(requests_file_path(app)?, contents)
        .map_err(|error| format!("Could not write saved requests: {error}"))
}

#[tauri::command]
fn load_requests(app: AppHandle) -> Result<Vec<StoredRequest>, String> {
    read_requests(&app)
}

#[tauri::command]
fn load_request(app: AppHandle, id: String) -> Result<StoredRequest, String> {
    read_requests(&app)?
        .into_iter()
        .find(|request| request.id == id)
        .ok_or_else(|| "The selected request no longer exists.".to_owned())
}

#[tauri::command]
fn save_request(
    app: AppHandle,
    id: String,
    name: String,
    method: String,
    domain: String,
    endpoint: String,
    body: String,
    headers: Vec<RequestHeader>,
) -> Result<StoredRequest, String> {
    let mut request = StoredRequest {
        id,
        name: name.trim().to_owned(),
        method,
        domain: domain.trim().to_owned(),
        endpoint: endpoint.trim().to_owned(),
        body,
        headers,
    };

    if request.name.is_empty() {
        return Err("A request name is required.".to_owned());
    }

    if request.id.is_empty() {
        request.id = format!(
            "request-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| format!("Could not generate a request ID: {error}"))?
                .as_millis()
        );
    }

    let mut requests = read_requests(&app)?;
    if let Some(position) = requests.iter().position(|saved| saved.id == request.id) {
        requests[position] = request.clone();
    } else {
        requests.push(request.clone());
    }

    write_requests(&app, &requests)?;
    Ok(request)
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
        .invoke_handler(tauri::generate_handler![
            load_requests,
            load_request,
            save_request,
            send_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
