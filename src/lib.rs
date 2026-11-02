use std::cell::Cell;

use kaja_web::prelude::*;
use wasm_bindgen::{JsCast, prelude::wasm_bindgen};
use wasm_bindgen_futures::spawn_local;

thread_local! {
    static HEADER_ROW_COUNT: Cell<usize> = const { Cell::new(1) };
}

#[derive(serde::Serialize)]
struct RequestHeader {
    key: String,
    value: String,
}

#[derive(serde::Serialize)]
struct PostRequestArgs {
    method: String,
    domain: String,
    endpoint: String,
    body: String,
    headers: Vec<RequestHeader>,
}

#[component("rest-call-app")]
struct RestCallApp {}

impl Component for RestCallAppComponent {
    fn connected(&self) {
        self.render();
    }
}

impl RestCallAppComponent {
    fn render(&self) {
        let id = self.get_id();

        let content = html! {{
            <main class="container">
                <header class="page-header">
                    <h1>REST Call</h1>
                    <p>Send a REST request to any HTTP server.</p>
                </header>

                <section class="request-form" aria-label="POST request">
                    <div class="field">
                        <label for="method-input">Method</label>
                        <select id="method-input">
                            <option value="GET" selected>GET</option>
                            <option value="POST">POST</option>
                            <option value="PUT">PUT</option>
                            <option value="PATCH">PATCH</option>
                            <option value="DELETE">DELETE</option>
                            <option value="HEAD">HEAD</option>
                            <option value="OPTIONS">OPTIONS</option>
                        </select>
                    </div>

                    <div class="field">
                        <label for="domain-input">Domain</label>
                        <input id="domain-input" type="url" placeholder="https://api.example.com" />
                    </div>

                    <div class="field">
                        <label for="endpoint-input">Endpoint</label>
                        <input id="endpoint-input" type="text" placeholder="/v1/resources" />
                    </div>

                    <div class="field">
                        <label for="body-input">Request body</label>
                        <textarea id="body-input" spellcheck="false" placeholder="{&quot;name&quot;:&quot;Example&quot;}" onkeydown="if (event.key === 'Tab') { event.preventDefault(); this.setRangeText('    ', this.selectionStart, this.selectionEnd, 'end'); }"></textarea>
                        <small>Press Tab to insert four spaces.</small>
                    </div>

                    <div class="field">
                        <span class="field-label">Headers</span>
                        <div id="header-list" class="header-list">
                            <div id="header-row-0" class="header-row">
                                <div class="header-fields">
                                    <div>
                                        <label for="header-key-input-0">Key</label>
                                        <input id="header-key-input-0" type="text" placeholder="Content-Type" />
                                    </div>
                                    <div>
                                        <label for="header-value-input-0">Value</label>
                                        <input id="header-value-input-0" type="text" placeholder="application/json" />
                                    </div>
                                </div>
                                <button onclick="removeHeader(0)">Remove</button>
                            </div>
                        </div>
                        <button onclick="addHeader()">Add header</button>
                    </div>

                    <button onclick="postRequest()">Send request</button>
                </section>

                <p id="request-status" role="status" aria-live="polite"></p>
                <pre id="request-result"></pre>
            </main>
        }};

        if let Some(element) = get_element_by_id(&id) {
            element.set_inner_html(content.as_str());
        }
    }
}

#[callback(addHeader)]
fn add_header_callback() {
    let added = HEADER_ROW_COUNT.with(|count| {
        let index = count.get();
        let row = html! {{
            <div id="header-row-$index" class="header-row">
                <div class="header-fields">
                    <div>
                        <label for="header-key-input-$index">Key</label>
                        <input id="header-key-input-$index" type="text">
                    </div>
                    <div>
                        <label for="header-value-input-$index">Value</label>
                        <input id="header-value-input-$index" type="text">
                    </div>
                </div>
                <button onclick="removeHeader($index)">Remove</button>
            </div>
        }};

        let Some(header_list) = get_element("#header-list") else {
            return false;
        };

        if header_list.insert_adjacent_html("beforeend", &row).is_ok() {
            count.set(index + 1);
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

#[callback(postRequest)]
fn post_request_callback() {
    let headers = match collect_headers() {
        Ok(headers) => headers,
        Err(message) => {
            try_inner_text("#request-status", &message);
            return;
        }
    };

    let payload = PostRequestArgs {
        method: try_get_select_value("#method-input"),
        domain: try_get_value("#domain-input").trim().to_owned(),
        endpoint: try_get_value("#endpoint-input").trim().to_owned(),
        body: try_get_textarea_value("#body-input"),
        headers,
    };

    if payload.domain.is_empty() {
        try_inner_text("#request-status", "Enter a domain to send the request.");
        return;
    }

    try_inner_text(
        "#request-status",
        &format!("Sending {} request…", payload.method),
    );
    try_inner_text("#request-result", "");

    spawn_local(async move {
        let args = match tauri_wasm::args(&payload) {
            Ok(args) => args,
            Err(error) => {
                try_inner_text(
                    "#request-status",
                    &format!("Could not prepare request: {:?}", error),
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
                    &format!("Could not read response: {:?}", error),
                ),
            },
            Err(error) => {
                try_inner_text("#request-status", &format!("Request failed: {:?}", error))
            }
        }
    });
}

#[wasm_bindgen(start)]
pub fn init() {
    init_callbacks();
}
