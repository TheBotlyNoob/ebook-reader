#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![warn(clippy::pedantic, clippy::nursery)]

mod app;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        initial_window_size: Some(egui::vec2(900., 700.)),
        ..Default::default()
    };
    eframe::run_native(
        "eBook Reader",
        options,
        Box::new(|_cc| Box::new(app::App::default())),
    );
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    wasm_bindgen_futures::spawn_local(async {
        let _ = eframe::start_web(
            "app",
            eframe::WebOptions::default(),
            Box::new(|_cc| Box::new(app::App::default())),
        )
        .await;
    });
}
