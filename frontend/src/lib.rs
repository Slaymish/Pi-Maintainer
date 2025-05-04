use leptos::*;
use wasm_bindgen::prelude::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    view! { cx,
        <div class="p-4 text-white bg-gray-900">"Hello from Leptos!"</div>
    }
}

#[wasm_bindgen(start)]
pub fn start() {
    mount_to_body(|cx| view! { cx, <App/> });
}

