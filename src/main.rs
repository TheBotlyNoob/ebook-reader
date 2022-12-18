mod app;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    tracing_subscriber::fmt::init();

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            console_error_panic_hook::set_once();
            tracing_wasm::set_as_global_default();

            dioxus::web::launch(app::root);
        } else if #[cfg(feature = "tui")] {
            dioxus::tui::launch(app::root);
        } else {
            dioxus::desktop::launch(app::root);
        }
    }
}
