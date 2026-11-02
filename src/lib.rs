use std::cell::Cell;

use kaja_web::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, prelude::wasm_bindgen};
use wasm_bindgen_futures::spawn_local;

mod tauri;

use tauri::{AddProjectArgs, LoadRequestArgs, LoadRequestListArgs, SaveRequestArgs};

thread_local! {
    static HEADER_ROW_COUNT: Cell<usize> = const { Cell::new(1) };
}

#[derive(Clone, Deserialize, Serialize)]
struct RequestHeader {
    key: String,
    value: String,
    enabled: bool,
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
    #[serde(default)]
    headers: Vec<RequestHeader>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestSummary {
    id: String,
    name: String,
    method: String,
    domain: String,
    endpoint: String,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectSummary {
    path: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PostResponse {
    method: String,
    status: u16,
    status_text: String,
    url: String,
    body: String,
}

#[component("app-root")]
struct App {
    #[attribute]
    view: String,
    #[attribute]
    request_id: String,
    #[attribute]
    request_name: String,
    #[attribute]
    request_method: String,
    #[attribute]
    request_domain: String,
    #[attribute]
    request_endpoint: String,
    #[attribute]
    request_body: String,
    #[attribute]
    request_headers: String,
    #[attribute]
    project_path: String,
    #[attribute]
    project_name: String,
}

#[component("rest-call-setup")]
struct RestCallSetup {
    #[attribute]
    request_id: String,
    #[attribute]
    request_name: String,
    #[attribute]
    request_method: String,
    #[attribute]
    request_domain: String,
    #[attribute]
    request_endpoint: String,
    #[attribute]
    request_body: String,
    #[attribute]
    request_headers: String,
    #[attribute]
    dirty: bool,
}

impl Component for AppComponent {
    fn connected(&self) {
        self.render();
    }

    fn attribute_changed(&self, name: &str, _old_value: &str, _new_value: &str) {
        if name == "view" {
            self.render();
        }
    }
}

impl AppComponent {
    fn render(&self) {
        match self.attribute("view").as_str() {
            "detail" => self.render_request_detail(),
            "picker" => {
                self.render_request_picker();
                load_requests();
            }
            _ => {
                self.render_project_picker();
                load_projects();
            }
        }
    }

    fn render_project_picker(&self) {
        let content = html! {{
            <main class="container">
                <header class="page-header page-header-split">
                    <div>
                        <h1>Projects</h1>
                        <p>Choose a folder that contains your saved requests.</p>
                    </div>
                    <div class="header-actions">
                        <button type="button" onclick="showProjectFolderDialog()">Add project</button>
                    </div>
                </header>
                <section class="request-picker" aria-label="Projects">
                    <div id="project-list" class="request-list" role="status" aria-live="polite">Loading projects...</div>
                </section>
            </main>

            <div id="project-folder-dialog" class="leave-dialog" role="dialog" aria-modal="true" aria-labelledby="project-folder-title" hidden>
                <div class="leave-dialog-panel">
                    <p class="eyebrow">Project folder</p>
                    <h2 id="project-folder-title">Choose a project folder</h2>
                    <p>Requests and headers will be saved in this folder.</p>
                    <div class="field project-folder-field">
                        <label for="project-folder-input">Folder path</label>
                        <div class="project-folder-input">
                            <input id="project-folder-input" type="text" placeholder="/path/to/project" />
                            <button class="button-secondary" type="button" onclick="browseProjectFolder()">Choose folder</button>
                        </div>
                    </div>
                    <p id="project-folder-error" class="request-status request-status-failed" role="alert" hidden></p>
                    <div class="leave-dialog-actions">
                        <button type="button" onclick="addProject()">Add project</button>
                        <button class="button-secondary" type="button" onclick="hideProjectFolderDialog()">Cancel</button>
                    </div>
                </div>
            </div>
        }};
        self.set_content(content.as_str());
    }

    fn render_request_picker(&self) {
        let project_name = self.attribute("project_name");
        let content = html! {{
            <main class="container">
                <header class="page-header page-header-split">
                    <div>
                        <h1>$project_name</h1>
                        <p>Choose a saved request or create a new one.</p>
                    </div>
                    <div class="header-actions">
                        <button class="button-secondary" type="button" onclick="showProjectPicker()">Back to projects</button>
                        <button type="button" onclick="newRequest()">New request</button>
                    </div>
                </header>
                <section class="request-picker" aria-label="Saved requests">
                    <div id="request-list" class="request-list" role="status" aria-live="polite">Loading saved requests...</div>
                </section>
            </main>
        }};
        self.set_content(content.as_str());
    }

    fn render_request_detail(&self) {
        let request_id = self.attribute("request_id");
        let request_name = self.attribute("request_name");
        let request_method = self.attribute("request_method");
        let request_domain = self.attribute("request_domain");
        let request_endpoint = self.attribute("request_endpoint");
        let request_body = self.attribute("request_body");
        let request_headers = self.attribute("request_headers");
        let content = html! {{
            <rest-call-setup
                request_id="$request_id"
                request_name="$request_name"
                request_method="$request_method"
                request_domain="$request_domain"
                request_endpoint="$request_endpoint"
                request_body="$request_body"
                request_headers="$request_headers">
            </rest-call-setup>
        }};
        self.set_content(content.as_str());
    }

    fn attribute(&self, name: &str) -> String {
        self.get_attribute_string(name).unwrap_or_default()
    }

    fn set_content(&self, content: &str) {
        if let Some(element) = get_element_by_id(&self.get_id()) {
            element.set_inner_html(content);
        }
    }
}

impl Component for RestCallSetupComponent {
    fn connected(&self) {
        self.render();
    }
}

impl RestCallSetupComponent {
    fn render(&self) {
        let headers = self.headers();
        let row_count = headers.len();
        HEADER_ROW_COUNT.with(|count| count.set(row_count));

        let mut header_rows = String::new();
        for index in 0..row_count {
            let header = headers.get(index);
            let key = header.map(|header| header.key.as_str()).unwrap_or_default();
            let value = header
                .map(|header| header.value.as_str())
                .unwrap_or_default();
            let enabled = header.map(|header| header.enabled).unwrap_or(true);
            header_rows.push_str(&header_row(index, key, value, enabled));
        }

        let request_id = self.attribute("request_id");
        let request_name = self.attribute("request_name");
        let request_name_headline = if request_name.trim().is_empty() {
            "None"
        } else {
            &request_name
        };
        let method = self.attribute("request_method");
        let domain = self.attribute("request_domain");
        let endpoint = self.attribute("request_endpoint");
        let body = self.attribute("request_body");
        let method = if method.is_empty() { "GET" } else { &method };
        let method_options = method_options(method);

        let content = html! {{
            <main class="container">
                <header class="page-header page-header-split">
                    <div>
                        <h1 id="request-title">$request_name_headline</h1>
                    </div>
                    <button class="button-secondary" type="button" onclick="showRequestPicker()">Back to requests</button>
                </header>

                <section class="request-form" aria-label="Request details">
                    <input id="request-id-input" type="hidden" value="$request_id" />
                    <div class="field">
                        <label for="request-name-input">Name</label>
                        <input id="request-name-input" type="text" value="$request_name" oninput="updateRequest()" />
                    </div>
                    <div class="field">
                        <label for="method-input">Method</label>
                        <select id="method-input" onchange="updateRequest()"></select>
                    </div>
                    <div class="field">
                        <label for="domain-input">Domain</label>
                        <input id="domain-input" type="url" value="$domain" placeholder="https://api.example.com" oninput="updateRequest()" />
                    </div>
                    <div class="field">
                        <label for="endpoint-input">Endpoint</label>
                        <input id="endpoint-input" type="text" value="$endpoint" placeholder="/v1/resources" oninput="updateRequest()" />
                    </div>
                    <div id="body-field" class="field">
                        <label for="body-input">Request body</label>
                        <textarea id="body-input" spellcheck="false" placeholder="{&quot;name&quot;:&quot;Example&quot;}" oninput="updateRequest()" onkeydown="handleBodyTab(event)">$body</textarea>
                    </div>
                    <div class="field">
                        <span class="field-label">Headers</span>
                        <div id="header-list" class="header-list"></div>
                        <button class="button-secondary" type="button" onclick="addHeader()">Add header</button>
                    </div>
                    <div class="form-actions">
                        <button type="button" onclick="saveRequest()">Save request</button>
                        <button class="button-secondary" type="button" onclick="postRequest()">Send request</button>
                    </div>
                </section>
                <p id="request-status" class="request-status" role="status" aria-live="polite"></p>
                <p id="request-url" class="request-url"></p>
                <section id="request-response" class="request-response" aria-label="Response" hidden>
                    <pre id="request-result"></pre>
                </section>
            </main>

            <div id="unsaved-changes-dialog" class="leave-dialog" role="dialog" aria-modal="true" aria-labelledby="unsaved-changes-title" hidden>
                <div class="leave-dialog-panel">
                    <p class="eyebrow">Unsaved changes</p>
                    <h2 id="unsaved-changes-title">Save this request before leaving?</h2>
                    <p>Your changes will be lost if you discard them.</p>
                    <div class="leave-dialog-actions">
                        <button type="button" onclick="saveAndShowRequestPicker()">Save</button>
                        <button class="button-danger" type="button" onclick="discardAndShowRequestPicker()">Discard changes</button>
                    </div>
                </div>
            </div>
        }};

        self.set_content(content.as_str());

        if let Some(method_input) = get_element("#method-input") {
            method_input.set_inner_html(&method_options);
        }
        if let Some(header_list) = get_element("#header-list") {
            header_list.set_inner_html(&header_rows);
        }
        update_header_list_visibility();
        update_request_url();
        update_request_body_visibility();
    }

    fn headers(&self) -> Vec<RequestHeader> {
        serde_json::from_str(&self.attribute("request_headers")).unwrap_or_default()
    }

    fn attribute(&self, name: &str) -> String {
        self.get_attribute_string(name).unwrap_or_default()
    }

    fn set_content(&self, content: &str) {
        if let Some(element) = get_element_by_id(&self.get_id()) {
            element.set_inner_html(content);
        }
    }
}

fn request_name_or_default(name: &str) -> &str {
    if name.trim().is_empty() {
        "New request"
    } else {
        name
    }
}

fn method_options(selected_method: &str) -> String {
    let mut options = String::new();
    for method in [
        "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "QUERY",
    ] {
        let selected = if method == selected_method {
            "selected"
        } else {
            ""
        };
        let option = html! {{
            <option value="$method" $selected>$method</option>
        }};
        options.push_str(option.as_str());
    }
    options
}

fn update_header_list_visibility() {
    let Some(header_list) = get_element("#header-list") else {
        return;
    };

    if get_element(".header-row").is_some() {
        let _ = header_list.remove_attribute("hidden");
    } else {
        let _ = header_list.set_attribute("hidden", "");
    }
}

fn header_row(index: usize, key: &str, value: &str, enabled: bool) -> String {
    let checked = if enabled { "checked" } else { "" };
    let row = html! {{
        <div id="header-row-$index" class="header-row">
            <input id="header-enabled-input-$index" class="header-enabled" type="checkbox" aria-label="Enable header" $checked onchange="updateRequest()" />
            <div class="header-fields">
                <div>
                    <label for="header-key-input-$index">Key</label>
                    <input id="header-key-input-$index" type="text" value="$key" oninput="updateRequest()" />
                </div>
                <div>
                    <label for="header-value-input-$index">Value</label>
                    <input id="header-value-input-$index" type="text" value="$value" oninput="updateRequest()" />
                </div>
            </div>
            <button class="button-danger" type="button" onclick="removeHeader($index)">Remove</button>
        </div>
    }};

    row.as_str().to_owned()
}

fn set_request_dirty(dirty: bool) {
    if let Some(request_app) = get_element("rest-call-setup") {
        let _ = request_app.set_attribute("dirty", if dirty { "true" } else { "false" });
    }
}

fn request_is_dirty() -> bool {
    get_element("rest-call-setup")
        .and_then(|request_app| request_app.get_attribute("dirty"))
        .as_deref()
        == Some("true")
}

fn set_request_status(message: &str) {
    if let Some(status) = get_element("#request-status") {
        let _ = status.set_attribute("class", "request-status");
    }
    try_inner_text("#request-status", message);
}

fn set_request_error(message: &str) {
    if let Some(status) = get_element("#request-status") {
        let _ = status.set_attribute("class", "request-status request-status-failed");
    }
    try_inner_text("#request-status", message);
}

fn set_request_status_with_code(status_code: u16, message: &str) {
    if let Some(status) = get_element("#request-status") {
        let class = if status_code == 200 {
            "request-status request-status-success"
        } else {
            "request-status request-status-failed"
        };
        let _ = status.set_attribute("class", class);
    }
    try_inner_text("#request-status", &format!("{status_code} {message}"));
}

fn display_request_url(method: &str, domain: &str, endpoint: &str) {
    let domain = domain.trim();
    if domain.is_empty() {
        try_inner_text("#request-url", "");
        return;
    }

    try_inner_text(
        "#request-url",
        &format!("{method} {domain}{}", endpoint.trim()),
    );
}

fn update_request_url() {
    display_request_url(
        &try_get_select_value("#method-input"),
        &try_get_value("#domain-input"),
        &try_get_value("#endpoint-input"),
    );
}

fn update_request_title() {
    let name = try_get_value("#request-name-input");
    let title = if name.trim().is_empty() {
        "None"
    } else {
        name.trim()
    };
    try_inner_text("#request-title", title);
}

fn update_request_body_visibility() {
    let Some(body_field) = get_element("#body-field") else {
        return;
    };
    let method = try_get_select_value("#method-input");

    if matches!(method.as_str(), "GET" | "HEAD") {
        let _ = body_field.set_attribute("hidden", "");
    } else {
        let _ = body_field.remove_attribute("hidden");
    }
}

fn selected_project_path() -> String {
    get_element("app-root")
        .and_then(|app| app.get_attribute("project_path"))
        .unwrap_or_default()
}

fn show_request_picker() {
    if let Some(app) = get_element("app-root") {
        let _ = app.set_attribute("view", "picker");
    }
}

fn show_project_picker() {
    if let Some(app) = get_element("app-root") {
        let _ = app.set_attribute("view", "projects");
    }
}

fn set_unsaved_changes_dialog_visible(visible: bool) {
    let Some(dialog) = get_element("#unsaved-changes-dialog") else {
        return;
    };

    if visible {
        let _ = dialog.remove_attribute("hidden");
    } else {
        let _ = dialog.set_attribute("hidden", "");
    }
}

fn set_request_attributes(request: &StoredRequest) {
    let Some(app) = get_element("app-root") else {
        return;
    };
    let headers = match serde_json::to_string(&request.headers) {
        Ok(headers) => headers,
        Err(_) => {
            set_request_error("Could not prepare request headers.");
            return;
        }
    };

    let _ = app.set_attribute("request_id", &request.id);
    let _ = app.set_attribute("request_name", &request.name);
    let _ = app.set_attribute("request_method", &request.method);
    let _ = app.set_attribute("request_domain", &request.domain);
    let _ = app.set_attribute("request_endpoint", &request.endpoint);
    let _ = app.set_attribute("request_body", &request.body);
    let _ = app.set_attribute("request_headers", &headers);
}

#[callback(loadProjects)]
fn load_projects() {
    spawn_local(async {
        match tauri_wasm::invoke("load_project_list").await {
            Ok(value) => match serde_wasm_bindgen::from_value::<Vec<ProjectSummary>>(value) {
                Ok(projects) => render_project_list(&projects),
                Err(error) => try_inner_text(
                    "#project-list",
                    &format!("Could not read projects: {error}"),
                ),
            },
            Err(error) => try_inner_text(
                "#project-list",
                &format!("Could not load projects: {error}"),
            ),
        }
    });
}

fn render_project_list(projects: &[ProjectSummary]) {
    let Some(app) = get_element("app-root") else {
        return;
    };
    if app.get_attribute("view").as_deref() == Some("picker")
        || app.get_attribute("view").as_deref() == Some("detail")
    {
        return;
    }

    if projects.is_empty() {
        try_inner_text(
            "#project-list",
            "No projects yet. Add a folder to get started.",
        );
        return;
    }

    let mut cards = String::new();
    for project in projects {
        let card = html! {{
            <button class="request-card" type="button" data-project-path="$(project.path)" data-project-name="$(project.name)" onclick="openProject(this.dataset.projectPath, this.dataset.projectName)">
                <span class="request-card-name">$(project.name)</span>
                <span class="request-card-details">$(project.path)</span>
            </button>
        }};
        cards.push_str(card.as_str());
    }

    if let Some(list) = get_element("#project-list") {
        list.set_inner_html(&cards);
    }
}

#[callback(loadRequests)]
fn load_requests() {
    let project_path = selected_project_path();
    if project_path.is_empty() {
        show_project_picker();
        return;
    }

    spawn_local(async move {
        let project_args = LoadRequestListArgs { project_path };
        let args = match tauri_wasm::args(&project_args) {
            Ok(args) => args,
            Err(error) => {
                try_inner_text(
                    "#request-list",
                    &format!("Could not load saved requests: {error}"),
                );
                return;
            }
        };

        match tauri_wasm::invoke("load_request_list")
            .with_args(args)
            .await
        {
            Ok(value) => match serde_wasm_bindgen::from_value::<Vec<RequestSummary>>(value) {
                Ok(requests) => render_request_list(&requests),
                Err(error) => try_inner_text(
                    "#request-list",
                    &format!("Could not read saved requests: {error}"),
                ),
            },
            Err(error) => try_inner_text(
                "#request-list",
                &format!("Could not load saved requests: {error}"),
            ),
        }
    });
}

fn render_request_list(requests: &[RequestSummary]) {
    let Some(app) = get_element("app-root") else {
        return;
    };

    if app.get_attribute("view").as_deref() != Some("picker") {
        return;
    }

    if requests.is_empty() {
        try_inner_text("#request-list", "No requests yet.");
        return;
    }

    let mut cards = String::new();

    for request in requests {
        let card = html! {{
            <button class="request-card" type="button" data-request-id="$(request.id)" onclick="openRequest(this.dataset.requestId)">
                <span class="request-card-name">$(request_name_or_default(&request.name))</span>
                <span class="request-card-details"><strong>$(request.method)</strong> $(request.domain)$(request.endpoint)</span>
            </button>
        }};
        cards.push_str(card.as_str());
    }

    if let Some(list) = get_element("#request-list") {
        list.set_inner_html(&cards);
    }
}

fn show_project_folder_dialog(start_new_request: bool) {
    let Some(dialog) = get_element("#project-folder-dialog") else {
        return;
    };

    let _ = dialog.set_attribute(
        "data-start-new-request",
        if start_new_request { "true" } else { "false" },
    );
    let _ = dialog.remove_attribute("hidden");
    clear_project_folder_error();
}

fn clear_project_folder_error() {
    try_inner_text("#project-folder-error", "");
    if let Some(error) = get_element("#project-folder-error") {
        let _ = error.set_attribute("hidden", "");
    }
}

fn hide_project_folder_dialog() {
    if let Some(dialog) = get_element("#project-folder-dialog") {
        let _ = dialog.set_attribute("hidden", "");
    }
}

fn set_project_folder_error(message: &str) {
    try_inner_text("#project-folder-error", message);
    if let Some(error) = get_element("#project-folder-error") {
        let _ = error.remove_attribute("hidden");
    }
}

fn set_project_folder_path(path: &str) {
    if let Some(input) = get_element("#project-folder-input")
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
    {
        input.set_value(path);
    }
}

fn select_project(project: &ProjectSummary) {
    if let Some(app) = get_element("app-root") {
        let _ = app.set_attribute("project_path", &project.path);
        let _ = app.set_attribute("project_name", &project.name);
    }
}

#[callback(showProjectFolderDialog)]
fn show_project_folder_dialog_action() {
    show_project_folder_dialog(false);
}

#[callback(hideProjectFolderDialog)]
fn hide_project_folder_dialog_action() {
    hide_project_folder_dialog();
}

#[callback(browseProjectFolder)]
fn browse_project_folder() {
    spawn_local(async {
        match tauri_wasm::invoke("pick_project_folder").await {
            Ok(value) => match serde_wasm_bindgen::from_value::<Option<String>>(value) {
                Ok(Some(path)) => {
                    set_project_folder_path(&path);
                    clear_project_folder_error();
                }
                Ok(None) => {}
                Err(error) => {
                    set_project_folder_error(&format!("Could not read selected folder: {error}"))
                }
            },
            Err(error) => set_project_folder_error(&format!("Could not choose a folder: {error}")),
        }
    });
}

#[callback(addProject)]
fn add_project() {
    let path = try_get_value("#project-folder-input").trim().to_owned();
    if path.is_empty() {
        set_project_folder_error("Enter a folder path.");
        return;
    }

    let Some(dialog) = get_element("#project-folder-dialog") else {
        return;
    };

    let start_new_request =
        dialog.get_attribute("data-start-new-request").as_deref() == Some("true");

    let project_args = AddProjectArgs { path };

    spawn_local(async move {
        let args = match tauri_wasm::args(&project_args) {
            Ok(args) => args,
            Err(error) => {
                set_project_folder_error(&format!("Could not prepare project folder: {error}"));
                return;
            }
        };

        match tauri_wasm::invoke("add_project").with_args(args).await {
            Ok(value) => match serde_wasm_bindgen::from_value::<ProjectSummary>(value) {
                Ok(project) => {
                    select_project(&project);
                    hide_project_folder_dialog();
                    if start_new_request {
                        new_request();
                    } else {
                        load_projects();
                    }
                }
                Err(error) => set_project_folder_error(&format!("Could not read project: {error}")),
            },
            Err(error) => set_project_folder_error(&format!("{error}")),
        }
    });
}

#[callback(newRequest)]
fn new_request() {
    if selected_project_path().is_empty() {
        show_project_folder_dialog(true);
        return;
    }

    set_request_attributes(&StoredRequest {
        id: String::new(),
        name: String::new(),
        method: "GET".to_owned(),
        domain: String::new(),
        endpoint: String::new(),
        body: String::new(),
        headers: Vec::new(),
    });
    if let Some(app) = get_element("app-root") {
        let _ = app.set_attribute("view", "detail");
    }
}

#[callback(updateRequest)]
fn update_request() {
    set_request_dirty(true);
    update_request_url();
    update_request_title();
    update_request_body_visibility();
}

#[callback(handleBodyTab)]
fn handle_body_tab(event: web_sys::KeyboardEvent) {
    if event.key() != "Tab" {
        return;
    }

    event.prevent_default();

    let element = get_element("#body-input");

    if element.is_none() {
        return;
    }

    let textarea = element.unwrap().dyn_into::<web_sys::HtmlTextAreaElement>();

    if textarea.is_err() {
        return;
    }

    let textarea = textarea.unwrap();

    let start = textarea
        .selection_start()
        .ok()
        .flatten()
        .unwrap_or_default();

    let end = textarea.selection_end().ok().flatten().unwrap_or(start);
    let _ = textarea.set_range_text_with_start_and_end_and_mode("    ", start, end, "end");
}

#[callback(showRequestPicker)]
fn show_request_picker_action() {
    if !request_is_dirty() {
        show_request_picker();
        return;
    }

    set_unsaved_changes_dialog_visible(true);
}

#[callback(saveAndShowRequestPicker)]
fn save_and_show_request_picker() {
    save_request(true);
}

#[callback(discardAndShowRequestPicker)]
fn discard_and_show_request_picker() {
    show_request_picker();
}

#[callback(showProjectPicker)]
fn show_project_picker_action() {
    if !request_is_dirty() {
        show_project_picker();
        return;
    }

    set_unsaved_changes_dialog_visible(true);
}

#[callback(openProject)]
fn open_project(path: String, name: String) {
    if let Some(app) = get_element("app-root") {
        let _ = app.set_attribute("project_path", &path);
        let _ = app.set_attribute("project_name", &name);
        let _ = app.set_attribute("view", "picker");
    }
}

#[callback(openRequest)]
fn open_request(id: String) {
    let project_path = selected_project_path();
    if project_path.is_empty() {
        show_project_picker();
        return;
    }

    spawn_local(async move {
        let request_args = LoadRequestArgs { project_path, id };
        let args = match tauri_wasm::args(&request_args) {
            Ok(args) => args,
            Err(error) => {
                try_inner_text("#request-list", &format!("Could not open request: {error}"));
                return;
            }
        };

        match tauri_wasm::invoke("load_request").with_args(args).await {
            Ok(value) => match serde_wasm_bindgen::from_value::<StoredRequest>(value) {
                Ok(request) => {
                    set_request_attributes(&request);
                    if let Some(app) = get_element("app-root") {
                        let _ = app.set_attribute("view", "detail");
                    }
                }
                Err(error) => try_inner_text(
                    "#request-list",
                    &format!("Could not read request details: {error}"),
                ),
            },
            Err(error) => {
                try_inner_text("#request-list", &format!("Could not open request: {error}"))
            }
        }
    });
}

#[callback(addHeader)]
fn add_header() {
    let added = HEADER_ROW_COUNT.with(|count| {
        let index = count.get();
        let Some(header_list) = get_element("#header-list") else {
            return false;
        };

        if header_list
            .insert_adjacent_html("beforeend", &header_row(index, "", "", true))
            .is_ok()
        {
            count.set(index + 1);
            update_header_list_visibility();
            set_request_dirty(true);
            true
        } else {
            false
        }
    });

    if !added {
        set_request_error("Could not add a header field.");
    }
}

#[callback(removeHeader)]
fn remove_header(index: usize) {
    let selector = format!("#header-row-{index}");
    if let Some(row) = get_element(&selector) {
        row.remove();
        update_header_list_visibility();
        set_request_dirty(true);
    }
}

fn try_get_textarea_value(selector: &str) -> String {
    get_element(selector)
        .and_then(|element| element.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
        .map(|textarea| textarea.value())
        .unwrap_or_default()
}

fn try_get_select_value(selector: &str) -> String {
    get_element(selector)
        .and_then(|element| element.dyn_into::<web_sys::HtmlSelectElement>().ok())
        .map(|select| select.value())
        .unwrap_or_default()
}

fn try_get_checkbox_checked(selector: &str) -> bool {
    get_element(selector)
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.checked())
        .unwrap_or(true)
}

fn collect_headers() -> std::result::Result<Vec<RequestHeader>, String> {
    HEADER_ROW_COUNT.with(|count| {
        let mut headers = Vec::new();
        for index in 0..count.get() {
            let key = try_get_value(&format!("#header-key-input-{index}"))
                .trim()
                .to_owned();
            let value = try_get_value(&format!("#header-value-input-{index}"))
                .trim()
                .to_owned();
            let enabled = try_get_checkbox_checked(&format!("#header-enabled-input-{index}"));

            if key.is_empty() && value.is_empty() {
                continue;
            }
            if enabled && (key.is_empty() || value.is_empty()) {
                return Err(format!("Header {} needs both a key and value.", index + 1));
            }
            headers.push(RequestHeader {
                key,
                value,
                enabled,
            });
        }
        Ok(headers)
    })
}

fn request_from_form() -> std::result::Result<StoredRequest, String> {
    Ok(StoredRequest {
        id: try_get_value("#request-id-input"),
        name: try_get_value("#request-name-input").trim().to_owned(),
        method: try_get_select_value("#method-input"),
        domain: try_get_value("#domain-input").trim().to_owned(),
        endpoint: try_get_value("#endpoint-input").trim().to_owned(),
        body: try_get_textarea_value("#body-input"),
        headers: collect_headers()?,
    })
}

fn clear_request_result() {
    if let Some(result) = get_element("#request-result") {
        let height = result.client_height();
        if height > 0 {
            let _ = result.set_attribute("style", &format!("min-height: {height}px"));
        }
    }
    if let Some(response) = get_element("#request-response") {
        let _ = response.set_attribute("class", "request-response request-response-empty");
        let _ = response.set_attribute("aria-hidden", "true");
    }
    try_inner_text("#request-result", "");
}

fn format_response_body(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .and_then(|value| serde_json::to_string_pretty(&value))
        .unwrap_or_else(|_| body.to_owned())
}

fn render_request_response(response: PostResponse) {
    try_inner_text(
        "#request-url",
        &format!("{} {}", response.method, response.url),
    );

    if response.body.trim().is_empty() {
        clear_request_result();
        return;
    }

    if let Some(response_container) = get_element("#request-response") {
        let _ = response_container.remove_attribute("hidden");
        let _ = response_container.set_attribute("class", "request-response");
        let _ = response_container.remove_attribute("aria-hidden");
    }

    try_inner_text("#request-result", &format_response_body(&response.body));
    if let Some(result) = get_element("#request-result") {
        let _ = result.remove_attribute("style");
    }
}

#[callback(saveRequest)]
fn save_request_action() {
    save_request(false);
}

#[callback(saveRequestShortcut)]
fn save_request_shortcut(event: web_sys::KeyboardEvent) {
    if !(event.ctrl_key() || event.meta_key())
        || !event.key().eq_ignore_ascii_case("s")
        || get_element("#request-name-input").is_none()
    {
        return;
    }

    event.prevent_default();
    save_request(false);
}

fn save_request(return_to_picker: bool) {
    let project_path = selected_project_path();
    if project_path.is_empty() {
        show_project_folder_dialog(true);
        return;
    }

    if try_get_value("#request-name-input").trim().is_empty() {
        set_request_error("Enter a request name before saving.");
        return;
    }

    let request = match request_from_form() {
        Ok(request) => request,
        Err(message) => {
            set_request_error(&message);
            return;
        }
    };

    set_request_status("Saving request");
    let request_args = SaveRequestArgs {
        project_path,
        request,
    };

    spawn_local(async move {
        let args = match tauri_wasm::args(&request_args) {
            Ok(args) => args,
            Err(error) => {
                set_request_error(&format!("Could not prepare request: {error}"));
                return;
            }
        };

        match tauri_wasm::invoke("save_request").with_args(args).await {
            Ok(value) => match serde_wasm_bindgen::from_value::<StoredRequest>(value) {
                Ok(saved_request) => {
                    set_request_attributes(&saved_request);
                    if let Some(id) = get_element("#request-id-input") {
                        let _ = id.set_attribute("value", &saved_request.id);
                    }
                    set_request_dirty(false);
                    clear_request_result();
                    set_request_status("Request saved.");
                    if return_to_picker {
                        show_request_picker();
                    }
                }
                Err(error) => set_request_error(&format!("Could not read saved request: {error}")),
            },
            Err(error) => set_request_error(&format!("Could not save request: {error}")),
        }
    });
}

#[callback(postRequest)]
fn post_request() {
    let payload = match request_from_form() {
        Ok(request) => request,
        Err(message) => {
            set_request_error(&message);
            return;
        }
    };
    display_request_url(&payload.method, &payload.domain, &payload.endpoint);

    if payload.domain.is_empty() {
        set_request_error("Enter a domain to send the request.");
        return;
    }

    set_request_status(&format!("Sending {} request…", payload.method));
    spawn_local(async move {
        let args = match tauri_wasm::args(&payload) {
            Ok(args) => args,
            Err(error) => {
                set_request_error(&format!("Could not prepare request: {error}"));
                return;
            }
        };

        match tauri_wasm::invoke("send_request").with_args(args).await {
            Ok(value) => match serde_wasm_bindgen::from_value::<PostResponse>(value) {
                Ok(response) => {
                    set_request_status_with_code(response.status, &response.status_text);
                    render_request_response(response);
                }
                Err(error) => set_request_error(&format!("Could not read response: {error}")),
            },
            Err(error) => set_request_error(&error.to_string()),
        }
    });
}

#[wasm_bindgen(start)]
pub fn init() {
    init_callbacks();

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(callback) = get_callback("saveRequestShortcut") else {
        return;
    };
    let _ = window.add_event_listener_with_callback("keydown", &callback);
}
