use std::{collections::BTreeMap, fs, io, path::PathBuf};

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
    enabled: bool,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestData {
    id: String,
    name: String,
    method: String,
    domain: String,
    endpoint: String,
    body: String,
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

type HeadersByRequestName = BTreeMap<String, Vec<RequestHeader>>;

const REQUESTS_FILE_NAME: &str = "requests.json";
const HEADERS_FILE_NAME: &str = "headers.json";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PostResponse {
    method: String,
    status: u16,
    status_text: String,
    url: String,
    body: String,
}

fn storage_file_path(app: &AppHandle, file_name: &str) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not find the application data directory: {error}"))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|error| format!("Could not create the application data directory: {error}"))?;

    Ok(app_data_dir.join(file_name))
}

fn requests_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    storage_file_path(app, REQUESTS_FILE_NAME)
}

fn headers_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    storage_file_path(app, HEADERS_FILE_NAME)
}

fn load_requests(app: &AppHandle) -> Result<Vec<RequestData>, String> {
    let path = requests_file_path(app)?;
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not read saved requests: {error}")),
    };

    serde_json::from_str(&contents)
        .map_err(|error| format!("Saved requests file is invalid JSON: {error}"))
}

fn write_requests(app: &AppHandle, requests: &[RequestData]) -> Result<(), String> {
    let contents = serde_json::to_string_pretty(requests)
        .map_err(|error| format!("Could not serialize saved requests: {error}"))?;
    fs::write(requests_file_path(app)?, contents)
        .map_err(|error| format!("Could not write saved requests: {error}"))
}

fn read_headers(app: &AppHandle) -> Result<HeadersByRequestName, String> {
    let path = headers_file_path(app)?;
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(HeadersByRequestName::new());
        }
        Err(error) => return Err(format!("Could not read saved request headers: {error}")),
    };

    serde_json::from_str(&contents)
        .map_err(|error| format!("Saved request headers file is invalid JSON: {error}"))
}

fn write_headers(app: &AppHandle, headers: &HeadersByRequestName) -> Result<(), String> {
    let contents = serde_json::to_string_pretty(headers)
        .map_err(|error| format!("Could not serialize saved request headers: {error}"))?;
    fs::write(headers_file_path(app)?, contents)
        .map_err(|error| format!("Could not write saved request headers: {error}"))
}

fn read_headers_for_request(
    app: &AppHandle,
    request_name: &str,
) -> Result<Vec<RequestHeader>, String> {
    Ok(read_headers(app)?
        .get(request_name)
        .cloned()
        .unwrap_or_default())
}

#[tauri::command]
fn load_request_list(app: AppHandle) -> Result<Vec<RequestData>, String> {
    load_requests(&app)
}

#[tauri::command]
fn load_request(app: AppHandle, id: String) -> Result<StoredRequest, String> {
    let request = load_requests(&app)?
        .into_iter()
        .find(|request| request.id == id)
        .ok_or_else(|| "The selected request no longer exists.".to_owned())?;
    let headers = read_headers_for_request(&app, &request.name)?;

    Ok(StoredRequest {
        id: request.id,
        name: request.name,
        method: request.method,
        domain: request.domain,
        endpoint: request.endpoint,
        body: request.body,
        headers,
    })
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
    let mut request = RequestData {
        id,
        name: name.trim().to_owned(),
        method,
        domain: domain.trim().to_owned(),
        endpoint: endpoint.trim().to_owned(),
        body,
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

    let mut requests = load_requests(&app)?;
    if requests
        .iter()
        .any(|saved| saved.id != request.id && saved.name == request.name)
    {
        return Err("A request with this name already exists.".to_owned());
    }

    let previous_name = requests
        .iter()
        .find(|saved| saved.id == request.id)
        .map(|saved| saved.name.clone());
    if let Some(position) = requests.iter().position(|saved| saved.id == request.id) {
        requests[position] = request.clone();
    } else {
        requests.push(request.clone());
    }

    let mut headers_by_name = read_headers(&app)?;
    headers_by_name.insert(request.name.clone(), headers.clone());

    if let Some(previous_name) =
        previous_name.filter(|previous_name| previous_name != &request.name)
    {
        if !requests.iter().any(|saved| saved.name == previous_name) {
            headers_by_name.remove(&previous_name);
        }
    }

    write_headers(&app, &headers_by_name)?;
    write_requests(&app, &requests)?;
    Ok(StoredRequest {
        id: request.id,
        name: request.name,
        method: request.method,
        domain: request.domain,
        endpoint: request.endpoint,
        body: request.body,
        headers,
    })
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
    let response_url = url.to_string();
    let headers = parse_headers(headers)?;

    let response_method = method.to_string();
    let method_allows_body = !matches!(method, Method::GET | Method::HEAD);
    let request = Client::new().request(method, url).headers(headers);
    let request = if method_allows_body {
        request.body(body)
    } else {
        request
    };
    let response = request
        .send()
        .await
        .map_err(|error| format!("HTTP request failed: {error}"))?;

    let status = response.status();
    let status_text = status
        .canonical_reason()
        .unwrap_or("Unknown status")
        .to_owned();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Could not read response body: {error}"))?;

    Ok(PostResponse {
        method: response_method,
        status: status.as_u16(),
        status_text,
        url: response_url,
        body,
    })
}

fn build_url(domain: &str, endpoint: &str) -> Result<Url, String> {
    let domain = domain.trim();
    if domain.is_empty() {
        return Err("Domain is required.".to_owned());
    }

    let base =
        Url::parse(domain).map_err(|_| "Domain must start with https:// or http://.".to_owned())?;
    if !matches!(base.scheme(), "http" | "https") || !base.has_host() {
        return Err("Domain must be an absolute http or https URL.".to_owned());
    }

    base.join(endpoint.trim())
        .map_err(|error| format!("Invalid endpoint: {error}"))
}

fn parse_headers(headers: Vec<RequestHeader>) -> Result<HeaderMap, String> {
    let mut header_map = HeaderMap::new();
    for header in headers {
        if !header.enabled {
            continue;
        }

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
            load_request_list,
            load_request,
            save_request,
            send_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
