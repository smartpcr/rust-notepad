//! Headless startup smoke test.
//!
//! Verifies that the app can be constructed and can run one frame of
//! `eframe::App::update` without panicking.  This catches runtime failures
//! (e.g. missing GPU, bad wgpu backend, wrong API calls) that `cargo build`
//! alone cannot detect.
//!
//! No display, GPU, or window is required — uses egui's built-in kittest
//! harness which runs entirely in-memory.

use eframe::egui;
use rust_notepad::app::RustNotepadApp;

/// Construct the app via the mock `CreationContext` and run one frame.
#[test]
fn app_starts_and_renders_one_frame_headless() {
    let ctx = egui::Context::default();
    let cc = eframe::CreationContext::_new_kittest(ctx.clone());

    let mut app = RustNotepadApp::new_with_files(&cc, vec![]);

    let mut frame = eframe::Frame::_new_kittest();

    // Run one full frame — panics here would indicate a runtime
    // incompatibility (bad layout, missing feature, etc.)
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1200.0, 800.0),
        )),
        ..Default::default()
    };

    let _ = ctx.run(input, |ctx| {
        eframe::App::update(&mut app, ctx, &mut frame);
    });
}

/// Verify the app can open a real file and render one frame.
#[test]
fn app_opens_file_and_renders_headless() {
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("hello.txt");
    std::fs::write(&file_path, "Hello, World!\nLine 2\n").expect("write file");

    let ctx = egui::Context::default();
    let cc = eframe::CreationContext::_new_kittest(ctx.clone());

    let mut app = RustNotepadApp::new_with_files(&cc, vec![file_path]);

    let mut frame = eframe::Frame::_new_kittest();

    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1200.0, 800.0),
        )),
        ..Default::default()
    };

    let _ = ctx.run(input, |ctx| {
        eframe::App::update(&mut app, ctx, &mut frame);
    });
}
