use std::cell::Cell;

use kaja_web::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, prelude::wasm_bindgen};
use wasm_bindgen_futures::spawn_local;

thread_local! {
    static HEADER_ROW_COUNT: Cell<usize> = const { Cell::new(1) };
}

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

#[derive(Serialize)]
struct LoadRequestArgs {
    id: String,
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
        if self.attribute("view") == "detail" {
            self.render_request_detail();
        } else {
            self.render_request_picker();
            load_requests_callback();
        }
    }

    fn render_request_picker(&self) {
        let content = r#"
            <main class="container">
                <header class="page-header page-header--split">
                    <div>
                        <h1>REST Call</h1>
                        <p>Choose a saved request or create a new one.</p>
                    </div>
                    <button type="button" onclick="newRequest()">New request</button>
                </header>
                <section class="request-picker" aria-label="Saved requests">
                    <div id="request-list" class="request-list" role="status" aria-live="polite">Loading saved requests…</div>
                </section>
            </main>
        "#;
        self.set_content(content);
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
            header_rows.push_str(&header_row(index, key, value));
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
                <header class="page-header page-header--split">
                    <div>
                        <h1>$request_name_headline</h1>
                    </div>
                    <button class="button-secondary" type="button" onclick="showRequestPicker()">All requests</button>
                </header>

                <section class="request-form" aria-label="Request details">
                    <input id="request-id-input" type="hidden" value="$request_id" />
                    <div class="field">
                        <label for="request-name-input">Name</label>
                        <input id="request-name-input" type="text" value="$request_name" placeholder="List users" oninput="markRequestDirty()" />
                    </div>
                    <div class="field">
                        <label for="method-input">Method</label>
                        <select id="method-input" onchange="markRequestDirty()"></select>
                    </div>
                    <div class="field">
                        <label for="domain-input">Domain</label>
                        <input id="domain-input" type="url" value="$domain" placeholder="https://api.example.com" oninput="markRequestDirty()" />
                    </div>
                    <div class="field">
                        <label for="endpoint-input">Endpoint</label>
                        <input id="endpoint-input" type="text" value="$endpoint" placeholder="/v1/resources" oninput="markRequestDirty()" />
                    </div>
                    <div class="field">
                        <label for="body-input">Request body</label>
                        <textarea id="body-input" spellcheck="false" placeholder="{&quot;name&quot;:&quot;Example&quot;}" oninput="markRequestDirty()" onkeydown="handleBodyTab(event)">$body</textarea>
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
                <p id="request-status" role="status" aria-live="polite"></p>
                <pre id="request-result"></pre>
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
    for method in ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
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

fn header_row(index: usize, key: &str, value: &str) -> String {
    let row = html! {{
        <div id="header-row-$index" class="header-row">
            <div class="header-fields">
                <div>
                    <label for="header-key-input-$index">Key</label>
                    <input id="header-key-input-$index" type="text" value="$key" oninput="markRequestDirty()" />
                </div>
                <div>
                    <label for="header-value-input-$index">Value</label>
                    <input id="header-value-input-$index" type="text" value="$value" oninput="markRequestDirty()" />
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

fn show_request_picker() {
    if let Some(app) = get_element("app-root") {
        let _ = app.set_attribute("view", "picker");
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
            try_inner_text("#request-status", "Could not prepare request headers.");
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

#[callback(loadRequests)]
fn load_requests_callback() {
    spawn_local(async {
        match tauri_wasm::invoke("load_requests").await {
            Ok(value) => match serde_wasm_bindgen::from_value::<Vec<StoredRequest>>(value) {
                Ok(requests) => render_request_list(&requests),
                Err(error) => try_inner_text(
                    "#request-list",
                    &format!("Could not read saved requests: {error}"),
                ),
            },
            Err(error) => try_inner_text(
                "#request-list",
                &format!("Could not load saved requests: {error:?}"),
            ),
        }
    });
}

fn render_request_list(requests: &[StoredRequest]) {
    let Some(app) = get_element("app-root") else {
        return;
    };

    if app.get_attribute("view").as_deref() == Some("detail") {
        return;
    }

    if requests.is_empty() {
        try_inner_text(
            "#request-list",
            "No saved requests yet. Create one to get started.",
        );
        return;
    }

    let mut cards = String::new();

    for request in requests {
        let request_id = &request.id;
        let request_name = request_name_or_default(&request.name);
        let request_method = &request.method;
        let request_domain = &request.domain;
        let request_endpoint = &request.endpoint;
        let card = html! {{
            <button class="request-card" type="button" data-request-id="$request_id" onclick="openRequest(this.dataset.requestId)">
                <span class="request-card-name">$request_name</span>
                <span class="request-card-details"><strong>$request_method</strong> $request_domain$request_endpoint</span>
            </button>
        }};
        cards.push_str(card.as_str());
    }

    if let Some(list) = get_element("#request-list") {
        list.set_inner_html(&cards);
    }
}

#[callback(newRequest)]
fn new_request_callback() {
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

#[callback(markRequestDirty)]
fn mark_request_dirty_callback() {
    set_request_dirty(true);
}

#[callback(handleBodyTab)]
fn handle_body_tab_callback(event: web_sys::KeyboardEvent) {
    if event.key() != "Tab" {
        return;
    }

    event.prevent_default();

    let Some(textarea) = get_element("#body-input")
        .and_then(|element| element.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
    else {
        return;
    };

    let start = textarea
        .selection_start()
        .ok()
        .flatten()
        .unwrap_or_default();
    let end = textarea.selection_end().ok().flatten().unwrap_or(start);
    let _ = textarea.set_range_text_with_start_and_end_and_mode("    ", start, end, "end");
}

#[callback(showRequestPicker)]
fn show_request_picker_callback() {
    if !request_is_dirty() {
        show_request_picker();
        return;
    }

    set_unsaved_changes_dialog_visible(true);
}

#[callback(saveAndShowRequestPicker)]
fn save_and_show_request_picker_callback() {
    save_request(true);
}

#[callback(discardAndShowRequestPicker)]
fn discard_and_show_request_picker_callback() {
    show_request_picker();
}

#[callback(openRequest)]
fn open_request_callback(id: String) {
    spawn_local(async move {
        let request_args = LoadRequestArgs { id };
        let args = match tauri_wasm::args(&request_args) {
            Ok(args) => args,
            Err(error) => {
                try_inner_text(
                    "#request-list",
                    &format!("Could not open request: {error:?}"),
                );
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
            Err(error) => try_inner_text(
                "#request-list",
                &format!("Could not open request: {error:?}"),
            ),
        }
    });
}

#[callback(addHeader)]
fn add_header_callback() {
    let added = HEADER_ROW_COUNT.with(|count| {
        let index = count.get();
        let Some(header_list) = get_element("#header-list") else {
            return false;
        };

        if header_list
            .insert_adjacent_html("beforeend", &header_row(index, "", ""))
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
        try_inner_text("#request-status", "Could not add a header field.");
    }
}

#[callback(removeHeader)]
fn remove_header_callback(index: usize) {
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

            if key.is_empty() && value.is_empty() {
                continue;
            }
            if key.is_empty() || value.is_empty() {
                return Err(format!("Header {} needs both a key and value.", index + 1));
            }
            headers.push(RequestHeader { key, value });
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

#[callback(saveRequest)]
fn save_request_callback() {
    save_request(false);
}

fn save_request(return_to_picker: bool) {
    let request = match request_from_form() {
        Ok(request) => request,
        Err(message) => {
            try_inner_text("#request-status", &message);
            return;
        }
    };

    if request.name.is_empty() {
        try_inner_text("#request-status", "Enter a request name before saving.");
        return;
    }

    try_inner_text("#request-status", "Saving request");

    spawn_local(async move {
        let args = match tauri_wasm::args(&request) {
            Ok(args) => args,
            Err(error) => {
                try_inner_text(
                    "#request-status",
                    &format!("Could not prepare request: {error:?}"),
                );
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
                    try_inner_text("#request-status", "Request saved.");
                    if return_to_picker {
                        show_request_picker();
                    }
                }
                Err(error) => try_inner_text(
                    "#request-status",
                    &format!("Could not read saved request: {error}"),
                ),
            },
            Err(error) => try_inner_text(
                "#request-status",
                &format!("Could not save request: {error:?}"),
            ),
        }
    });
}

#[callback(postRequest)]
fn post_request_callback() {
    let payload = match request_from_form() {
        Ok(request) => request,
        Err(message) => {
            try_inner_text("#request-status", &message);
            return;
        }
    };
    if payload.domain.is_empty() {
        try_inner_text("#request-status", "Enter a domain to send the request.");
        return;
    }

    try_inner_text(
        &"#request-status",
        &format!("Sending {} request…", payload.method),
    );
    try_inner_text("#request-result", "");
    spawn_local(async move {
        let args = match tauri_wasm::args(&payload) {
            Ok(args) => args,
            Err(error) => {
                try_inner_text(
                    "#request-status",
                    &format!("Could not prepare request: {error:?}"),
                );
                return;
            }
        };

        match tauri_wasm::invoke("send_request").with_args(args).await {
            Ok(value) => match js_sys::JSON::stringify(&value) {
                Ok(response) => {
                    try_inner_text("#request-status", "Request completed.");
                    try_inner_text("#request-result", &response.as_string().unwrap_or_default());
                }
                Err(error) => try_inner_text(
                    "#request-status",
                    &format!("Could not read response: {error:?}"),
                ),
            },
            Err(error) => try_inner_text("#request-status", &format!("Request failed: {error:?}")),
        }
    });
}

#[wasm_bindgen(start)]
pub fn init() {
    init_callbacks();
}
