#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use tracing_subscriber::{
            filter::filter_fn, layer::SubscriberExt, util::SubscriberInitExt, Layer,
        };

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .without_time()
                    .with_filter(filter_fn(|m| {
                        m.module_path()
                            .map(|m| !m.contains("wgpu"))
                            .unwrap_or(false)
                            && m.level() <= &tracing::Level::INFO
                    })),
            )
            .init();
        dioxus_desktop::launch(ebook_reader::app);
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        tracing_wasm::set_as_global_default();
        dioxus_web::launch(ebook_reader::app);
    }
}
