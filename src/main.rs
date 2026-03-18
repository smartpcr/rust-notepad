#![windows_subsystem = "windows"]

use std::path::PathBuf;

use clap::Parser;
use log::error;
use rust_notepad::app::RustNotepadApp;

/// TextEdit — A Notepad++ clone in Rust
#[derive(Parser)]
#[command(name = "TextEdit", version, about)]
struct Cli {
    /// Files to open on startup
    files: Vec<PathBuf>,
}

fn setup_logging() {
    use std::io::Write;

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("textedit.log")
        .expect("Failed to open log file");

    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                record.args()
            )
        })
        .init();
}

fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let msg = format!("PANIC: {panic_info}");
        error!("{msg}");
        // Also write directly to the log file in case the logger is broken
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("textedit.log")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "[PANIC] {msg}")
            });
    }));
}

fn main() -> eframe::Result<()> {
    setup_logging();
    setup_panic_hook();
    log::info!("TextEdit starting up");

    let cli = Cli::parse();
    let files = cli.files;

    let viewport = eframe::egui::ViewportBuilder::default()
        .with_title("TextEdit")
        .with_inner_size([1200.0, 800.0])
        .with_drag_and_drop(true);

    // Try glow (OpenGL) first — it's faster on most machines.
    // Fall back to wgpu if OpenGL 2.0+ is not available.
    let glow_options = eframe::NativeOptions {
        viewport: viewport.clone(),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let files_clone = files.clone();
    let result = eframe::run_native(
        "TextEdit",
        glow_options,
        Box::new(move |cc| Ok(Box::new(RustNotepadApp::new_with_files(cc, files_clone)))),
    );

    if result.is_ok() {
        return result;
    }

    log::warn!(
        "Glow (OpenGL) failed, falling back to wgpu: {}",
        result.as_ref().unwrap_err()
    );

    let wgpu_options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    let result = eframe::run_native(
        "TextEdit",
        wgpu_options,
        Box::new(move |cc| Ok(Box::new(RustNotepadApp::new_with_files(cc, files)))),
    );

    if let Err(ref e) = result {
        error!("eframe exited with error: {e}");
    }

    result
}
