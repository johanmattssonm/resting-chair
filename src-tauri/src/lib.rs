use std::{collections::BTreeMap, fs, io, path::PathBuf};

use reqwest::{
    Client, Method, Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

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

#[derive(Clone, Deserialize, Serialize)]
struct Project {
    path: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSummary {
    path: String,
    name: String,
}

type HeadersByRequestName = BTreeMap<String, Vec<RequestHeader>>;

const PROJECTS_FILE_NAME: &str = "projects.json";
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

fn projects_file_path(app: &AppHandle) -> Result<PathBuf, String> {
    let config_dir = app.path().app_config_dir().map_err(|error| {
        format!("Could not find the application configuration directory: {error}")
    })?;

    fs::create_dir_all(&config_dir).map_err(|error| {
        format!("Could not create the application configuration directory: {error}")
    })?;

    Ok(config_dir.join(PROJECTS_FILE_NAME))
}

fn project_file_path(project_path: &str, file_name: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(project_path);
    if !path.is_dir() {
        return Err("The selected project folder no longer exists.".to_owned());
    }

    Ok(path.join(file_name))
}

fn requests_file_path(project_path: &str) -> Result<PathBuf, String> {
    project_file_path(project_path, REQUESTS_FILE_NAME)
}

fn headers_file_path(project_path: &str) -> Result<PathBuf, String> {
    project_file_path(project_path, HEADERS_FILE_NAME)
}

fn load_projects(app: &AppHandle) -> Result<Vec<Project>, String> {
    let path = projects_file_path(app)?;
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not read projects: {error}")),
    };

    serde_json::from_str(&contents)
        .map_err(|error| format!("Projects file is invalid JSON: {error}"))
}

fn write_projects(app: &AppHandle, projects: &[Project]) -> Result<(), String> {
    let contents = serde_json::to_string_pretty(projects)
        .map_err(|error| format!("Could not serialize projects: {error}"))?;
    fs::write(projects_file_path(app)?, contents)
        .map_err(|error| format!("Could not write projects: {error}"))
}

fn project_summary(project: &Project) -> ProjectSummary {
    let path = PathBuf::from(&project.path);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(&project.path)
        .to_owned();

    ProjectSummary {
        path: project.path.clone(),
        name,
    }
}

fn load_requests(project_path: &str) -> Result<Vec<RequestData>, String> {
    let path = requests_file_path(project_path)?;
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Could not read saved requests: {error}")),
    };

    serde_json::from_str(&contents)
        .map_err(|error| format!("Saved requests file is invalid JSON: {error}"))
}

fn write_requests(project_path: &str, requests: &[RequestData]) -> Result<(), String> {
    let contents = serde_json::to_string_pretty(requests)
        .map_err(|error| format!("Could not serialize saved requests: {error}"))?;
    fs::write(requests_file_path(project_path)?, contents)
        .map_err(|error| format!("Could not write saved requests: {error}"))
}

fn read_headers(project_path: &str) -> Result<HeadersByRequestName, String> {
    let path = headers_file_path(project_path)?;
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

fn write_headers(project_path: &str, headers: &HeadersByRequestName) -> Result<(), String> {
    let contents = serde_json::to_string_pretty(headers)
        .map_err(|error| format!("Could not serialize saved request headers: {error}"))?;
    fs::write(headers_file_path(project_path)?, contents)
        .map_err(|error| format!("Could not write saved request headers: {error}"))
}

fn read_headers_for_request(
    project_path: &str,
    request_name: &str,
) -> Result<Vec<RequestHeader>, String> {
    Ok(read_headers(project_path)?
        .get(request_name)
        .cloned()
        .unwrap_or_default())
}

#[tauri::command]
fn load_project_list(app: AppHandle) -> Result<Vec<ProjectSummary>, String> {
    let mut projects = load_projects(&app)?;
    let original_count = projects.len();
    projects.retain(|project| PathBuf::from(&project.path).is_dir());
    projects.sort_by(|left, right| left.path.cmp(&right.path));
    projects.dedup_by(|left, right| left.path == right.path);

    if projects.len() != original_count {
        write_projects(&app, &projects)?;
    }

    Ok(projects.iter().map(project_summary).collect())
}

#[tauri::command]
async fn pick_project_folder(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .set_title("Choose a project folder")
        .blocking_pick_folder()
        .map(|path| path.to_string())
}

#[tauri::command]
fn add_project(app: AppHandle, path: String) -> Result<ProjectSummary, String> {
    let canonical_path = fs::canonicalize(path.trim())
        .map_err(|error| format!("Could not access the project folder: {error}"))?;
    if !canonical_path.is_dir() {
        return Err("Choose an existing folder for the project.".to_owned());
    }

    let project = Project {
        path: canonical_path.to_string_lossy().into_owned(),
    };
    let mut projects = load_projects(&app)?;
    if !projects.iter().any(|saved| saved.path == project.path) {
        projects.push(project.clone());
        write_projects(&app, &projects)?;
    }

    Ok(project_summary(&project))
}

#[tauri::command]
fn load_request_list(project_path: String) -> Result<Vec<RequestData>, String> {
    load_requests(&project_path)
}

#[tauri::command]
fn load_request(project_path: String, id: String) -> Result<StoredRequest, String> {
    let request = load_requests(&project_path)?
        .into_iter()
        .find(|request| request.id == id)
        .ok_or_else(|| "The selected request no longer exists.".to_owned())?;
    let headers = read_headers_for_request(&project_path, &request.name)?;

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
    project_path: String,
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

    let mut requests = load_requests(&project_path)?;
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

    let mut headers_by_name = read_headers(&project_path)?;
    headers_by_name.insert(request.name.clone(), headers.clone());

    if let Some(previous_name) =
        previous_name.filter(|previous_name| previous_name != &request.name)
    {
        if !requests.iter().any(|saved| saved.name == previous_name) {
            headers_by_name.remove(&previous_name);
        }
    }

    write_headers(&project_path, &headers_by_name)?;
    write_requests(&project_path, &requests)?;
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
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_project_list,
            pick_project_folder,
            add_project,
            load_request_list,
            load_request,
            save_request,
            send_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
