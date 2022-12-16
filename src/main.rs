mod app;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    dioxus::desktop::launch(app::root);
    #[cfg(target_arch = "wasm32")]
    dioxus::web::launch(app::root);
}
