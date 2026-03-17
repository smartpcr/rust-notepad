mod app;

use std::path::PathBuf;

use app::RustNotepadApp;
use clap::Parser;

/// TextEdit — A Notepad++ clone in Rust
#[derive(Parser)]
#[command(name = "TextEdit", version, about)]
struct Cli {
    /// Files to open on startup
    files: Vec<PathBuf>,
}

fn main() -> eframe::Result<()> {
    let cli = Cli::parse();
    let files = cli.files;

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("TextEdit")
            .with_inner_size([1200.0, 800.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "TextEdit",
        native_options,
        Box::new(move |cc| Ok(Box::new(RustNotepadApp::new_with_files(cc, files)))),
    )
}
