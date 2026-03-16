mod app;

use app::RustNotepadApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("CodeEdit")
            .with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "CodeEdit",
        native_options,
        Box::new(|cc| Ok(Box::new(RustNotepadApp::new(cc)))),
    )
}
