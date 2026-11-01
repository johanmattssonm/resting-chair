use kaja_web::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::spawn_local;

#[derive(serde::Serialize)]
struct GreetArgs {
    name: String,
}

#[component("greet-app")]
struct GreetApp {}

impl Component for GreetAppComponent {
    fn connected(&self) {
        self.render();
    }
}

impl GreetAppComponent {
    fn render(&self) {
        let id = self.get_id();

        let content = html! {{
            <main class="container">
                <h1>Welcome to Tauri</h1>

                <div class="row">
                    <a href="https://tauri.app" target="_blank">
                        <img src="/assets/tauri.svg" class="logo tauri" alt="Tauri logo" />
                    </a>
                </div>

                <p>Click on the Tauri logo to learn more about the framework</p>

                <div class="row" id="greet-form">
                    <input id="greet-input" placeholder="Enter a name..." />
                    <button type="button" onclick="greet()">Greet</button>
                </div>
                <p id="greet-result"></p>
            </main>
        }};

        if let Some(element) = get_element_by_id(&id) {
            element.set_inner_html(content.as_str());
        }
    }
}

#[callback(greet)]
fn greet_callback() {
    let name = try_get_value("#greet-input");

    if name.is_empty() {
        log!("No name provided.");
        return;
    }

    spawn_local(async move {
        let result = async {
            let payload = GreetArgs { name };
            let args = tauri_wasm::args(&payload)?;
            tauri_wasm::invoke("greet").with_args(args).await
        }
        .await;

        match result {
            Ok(value) => {
                let message = value.as_string().unwrap_or_default();
                try_inner_text("#greet-result", &message);
            }
            Err(error) => {
                let message = format!("Error: {:?}", error);
                try_inner_text("#greet-result", &message);
            }
        }
    });
}

#[wasm_bindgen(start)]
pub fn init() {
    init_callbacks();
}
