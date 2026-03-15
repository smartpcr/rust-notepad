mod app;

use app::RustNotepadApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Rust Notepad++-style Editor",
        native_options,
        Box::new(|cc| Ok(Box::new(RustNotepadApp::new(cc)))),
    )
}
