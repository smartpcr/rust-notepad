use std::{fs, time::Duration};

use rust_notepad::phase3::{
    line_diff, parse_manifest, project_search_with_options, render_diff, CommandRegistry,
    DiffRenderMode, MacroRecorder, PluginHost, PluginRpcRequest, PluginRpcResponse,
    PluginTransport, SearchOptions, SplitDirection, SplitLayout,
};
use serde_json::json;
use tempfile::tempdir;

struct E2ePluginTransport;

impl PluginTransport for E2ePluginTransport {
    fn send_json(&mut self, payload: &str) -> rust_notepad::phase0::AppResult<String> {
        let req: PluginRpcRequest =
            serde_json::from_str(payload).expect("plugin request should be valid json-rpc");

        let response = PluginRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id,
            result: Some(json!({
                "command": req.method,
                "preview": req.params["text"].as_str().unwrap_or_default().to_uppercase()
            })),
            error: None,
        };

        serde_json::to_string(&response)
            .map_err(|err| rust_notepad::phase0::AppError::Validation(err.to_string()))
    }
}

#[test]
fn phase3_end_to_end_flow_works() {
    let root = tempdir().expect("tempdir should be created");
    fs::create_dir_all(root.path().join("src")).expect("source dir should be created");
    fs::create_dir_all(root.path().join("target")).expect("target dir should be created");
    fs::write(
        root.path().join("src").join("main.rs"),
        "fn main() {\n    println!(\"hello\");\n}\n",
    )
    .expect("source file should be written");
    fs::write(
        root.path().join("target").join("ignored.rs"),
        "println!(\"hello\");",
    )
    .expect("ignored file should be written");

    let options = SearchOptions {
        include_extensions: vec!["rs".to_string()],
        exclude_dirs: vec!["target".to_string()],
        max_results: None,
    };
    let hits = project_search_with_options(root.path(), "println", &options)
        .expect("project search should complete");
    assert_eq!(hits.len(), 1);
    assert!(hits[0].file.ends_with("main.rs"));

    let original = "fn main() {\n    println!(\"hello\");\n}";
    let modified = "fn main() {\n    println!(\"hello phase 3\");\n}";
    let diff = line_diff(original, modified);
    let rendered = render_diff(&diff, DiffRenderMode::SideBySide);
    assert_eq!(rendered.len(), 3);
    assert_eq!(rendered[1].0.as_deref(), Some("    println!(\"hello\");"));
    assert_eq!(
        rendered[1].1.as_deref(),
        Some("    println!(\"hello phase 3\");")
    );

    let mut split = SplitLayout::new(SplitDirection::SideBySide, 0, 1);
    split.set_cursor(0, 15);
    split.set_cursor(1, 22);
    split.focus(1);
    assert_eq!(split.active_pane, 1);
    assert_eq!(split.panes[0].cursor_offset, 15);
    assert_eq!(split.panes[1].cursor_offset, 22);

    let manifest = parse_manifest(
        r#"{"name":"fmt","version":"1.0.0","command":"fmt-plugin","capabilities":["format"]}"#,
    )
    .expect("manifest should parse");
    let mut host = PluginHost::new(manifest, Duration::from_secs(1));
    host.start();
    let mut transport = E2ePluginTransport;
    let plugin_result = host
        .invoke(
            &mut transport,
            "format",
            json!({"text":"println!(\"hello\");"}),
        )
        .expect("plugin should return a format preview");
    assert_eq!(plugin_result["command"], "format");
    assert_eq!(plugin_result["preview"], "PRINTLN!(\"HELLO\");");

    let mut registry = CommandRegistry::default();
    registry.register("search.project", "Search in project");
    registry.register("view.split", "Toggle split view");
    let palette_matches = registry.search("split");
    assert_eq!(palette_matches, vec!["view.split".to_string()]);

    let mut macro_recorder = MacroRecorder::default();
    macro_recorder.start();
    macro_recorder.push("search.project");
    macro_recorder.push("view.split");
    macro_recorder.push("plugin.format");
    let replay = macro_recorder.stop();
    assert_eq!(
        replay,
        vec!["search.project", "view.split", "plugin.format"]
    );
}
