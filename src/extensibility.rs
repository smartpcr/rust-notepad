use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::core::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub command: String,
    pub capabilities: Vec<String>,
}

pub fn parse_manifest(raw: &str) -> AppResult<PluginManifest> {
    serde_json::from_str(raw).map_err(|err| AppError::Validation(err.to_string()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

pub trait PluginTransport {
    fn send_json(&mut self, payload: &str) -> AppResult<String>;
}

#[derive(Debug)]
pub struct PluginHost {
    manifest: PluginManifest,
    timeout: Duration,
    running: bool,
    next_id: u64,
}

impl PluginHost {
    pub fn new(manifest: PluginManifest, timeout: Duration) -> Self {
        Self {
            manifest,
            timeout,
            running: false,
            next_id: 1,
        }
    }

    pub fn start(&mut self) {
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn invoke(
        &mut self,
        transport: &mut dyn PluginTransport,
        method: impl Into<String>,
        params: serde_json::Value,
    ) -> AppResult<serde_json::Value> {
        if !self.running {
            return Err(AppError::Validation(
                "plugin host is not running".to_string(),
            ));
        }

        let method = method.into();
        if !self.manifest.capabilities.contains(&method) {
            return Err(AppError::Validation(format!(
                "plugin '{}' does not expose capability '{method}'",
                self.manifest.name
            )));
        }

        let req_id = self.next_id;
        self.next_id += 1;

        let req = PluginRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: req_id,
            method,
            params,
        };

        let raw_req =
            serde_json::to_string(&req).map_err(|err| AppError::Validation(err.to_string()))?;
        let raw_resp = transport.send_json(&raw_req)?;
        let response: PluginRpcResponse =
            serde_json::from_str(&raw_resp).map_err(|err| AppError::Validation(err.to_string()))?;

        if response.id != req_id {
            return Err(AppError::Validation(
                "plugin response id mismatch".to_string(),
            ));
        }

        if let Some(error) = response.error {
            return Err(AppError::Validation(error));
        }

        response
            .result
            .ok_or_else(|| AppError::Validation("plugin response missing result".to_string()))
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectHit {
    pub file: PathBuf,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchOptions {
    pub include_extensions: Vec<String>,
    pub exclude_dirs: Vec<String>,
    pub max_results: Option<usize>,
}

pub fn project_search(root: &Path, needle: &str) -> AppResult<Vec<ProjectHit>> {
    project_search_with_options(root, needle, &SearchOptions::default())
}

pub fn project_search_with_options(
    root: &Path,
    needle: &str,
    options: &SearchOptions,
) -> AppResult<Vec<ProjectHit>> {
    if needle.is_empty() {
        return Ok(Vec::new());
    }

    let mut hits = Vec::new();
    let _ = project_search_stream(root, needle, options, |hit| hits.push(hit))?;
    Ok(hits)
}

pub fn project_search_stream<F: FnMut(ProjectHit)>(
    root: &Path,
    needle: &str,
    options: &SearchOptions,
    mut on_hit: F,
) -> AppResult<usize> {
    if needle.is_empty() {
        return Ok(0);
    }

    let mut total_hits = 0;
    project_search_walk(root, needle, options, &mut total_hits, &mut on_hit)?;
    Ok(total_hits)
}

fn project_search_walk<F: FnMut(ProjectHit)>(
    root: &Path,
    needle: &str,
    options: &SearchOptions,
    total_hits: &mut usize,
    on_hit: &mut F,
) -> AppResult<()> {
    for entry in fs::read_dir(root).map_err(|err| AppError::Validation(err.to_string()))? {
        let entry = entry.map_err(|err| AppError::Validation(err.to_string()))?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if options
                .exclude_dirs
                .iter()
                .any(|excluded| excluded == dir_name)
            {
                continue;
            }

            project_search_walk(&path, needle, options, total_hits, on_hit)?;
            continue;
        }

        if !options.include_extensions.is_empty() {
            let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
            if !options
                .include_extensions
                .iter()
                .any(|candidate| candidate == ext)
            {
                continue;
            }
        }

        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };

        for (line_idx, line) in content.lines().enumerate() {
            if line.contains(needle) {
                *total_hits += 1;
                on_hit(ProjectHit {
                    file: path.clone(),
                    line: line_idx + 1,
                    text: line.to_owned(),
                });

                if options
                    .max_results
                    .is_some_and(|max_results| *total_hits >= max_results)
                {
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffOp {
    Unchanged(String),
    Added(String),
    Removed(String),
}

pub fn line_diff(old: &str, new: &str) -> Vec<DiffOp> {
    let old_lines: Vec<_> = old.lines().collect();
    let new_lines: Vec<_> = new.lines().collect();
    let mut out = Vec::new();

    let max_len = old_lines.len().max(new_lines.len());
    for idx in 0..max_len {
        match (old_lines.get(idx), new_lines.get(idx)) {
            (Some(a), Some(b)) if a == b => out.push(DiffOp::Unchanged((*a).to_owned())),
            (Some(a), Some(b)) => {
                out.push(DiffOp::Removed((*a).to_owned()));
                out.push(DiffOp::Added((*b).to_owned()));
            }
            (Some(a), None) => out.push(DiffOp::Removed((*a).to_owned())),
            (None, Some(b)) => out.push(DiffOp::Added((*b).to_owned())),
            (None, None) => {}
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffRenderMode {
    Inline,
    SideBySide,
}

pub fn render_diff(ops: &[DiffOp], mode: DiffRenderMode) -> Vec<(Option<String>, Option<String>)> {
    match mode {
        DiffRenderMode::Inline => ops
            .iter()
            .map(|op| match op {
                DiffOp::Unchanged(line) => (Some(format!(" {line}")), None),
                DiffOp::Removed(line) => (Some(format!("-{line}")), None),
                DiffOp::Added(line) => (Some(format!("+{line}")), None),
            })
            .collect(),
        DiffRenderMode::SideBySide => {
            let mut rows = Vec::new();
            let mut idx = 0;
            while idx < ops.len() {
                match (&ops[idx], ops.get(idx + 1)) {
                    (DiffOp::Removed(left), Some(DiffOp::Added(right))) => {
                        rows.push((Some(left.clone()), Some(right.clone())));
                        idx += 2;
                    }
                    (DiffOp::Unchanged(line), _) => {
                        rows.push((Some(line.clone()), Some(line.clone())));
                        idx += 1;
                    }
                    (DiffOp::Removed(line), _) => {
                        rows.push((Some(line.clone()), None));
                        idx += 1;
                    }
                    (DiffOp::Added(line), _) => {
                        rows.push((None, Some(line.clone())));
                        idx += 1;
                    }
                }
            }
            rows
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    SideBySide,
    Stacked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneState {
    pub tab_index: usize,
    pub cursor_offset: usize,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitLayout {
    pub direction: SplitDirection,
    pub panes: [PaneState; 2],
    pub active_pane: usize,
}

impl SplitLayout {
    pub fn new(direction: SplitDirection, left_tab_index: usize, right_tab_index: usize) -> Self {
        Self {
            direction,
            panes: [
                PaneState {
                    tab_index: left_tab_index,
                    cursor_offset: 0,
                    scroll_offset: 0,
                },
                PaneState {
                    tab_index: right_tab_index,
                    cursor_offset: 0,
                    scroll_offset: 0,
                },
            ],
            active_pane: 0,
        }
    }

    pub fn focus(&mut self, pane_index: usize) {
        if pane_index < self.panes.len() {
            self.active_pane = pane_index;
        }
    }

    pub fn set_cursor(&mut self, pane_index: usize, cursor_offset: usize) {
        if let Some(pane) = self.panes.get_mut(pane_index) {
            pane.cursor_offset = cursor_offset;
        }
    }

    pub fn set_scroll(&mut self, pane_index: usize, scroll_offset: usize) {
        if let Some(pane) = self.panes.get_mut(pane_index) {
            pane.scroll_offset = scroll_offset;
        }
    }
}

#[derive(Debug, Default)]
pub struct CommandRegistry {
    commands: HashMap<String, String>,
}

impl CommandRegistry {
    pub fn register(&mut self, id: impl Into<String>, description: impl Into<String>) {
        self.commands.insert(id.into(), description.into());
    }

    pub fn search(&self, query: &str) -> Vec<String> {
        let mut matches: Vec<_> = self
            .commands
            .iter()
            .filter(|(id, desc)| id.contains(query) || desc.contains(query))
            .map(|(id, _)| id.clone())
            .collect();
        matches.sort();
        matches
    }
}

#[derive(Debug, Default)]
pub struct MacroRecorder {
    recording: bool,
    captured: Vec<String>,
}

impl MacroRecorder {
    pub fn start(&mut self) {
        self.recording = true;
        self.captured.clear();
    }

    pub fn push(&mut self, command: impl Into<String>) {
        if self.recording {
            self.captured.push(command.into());
        }
    }

    pub fn stop(&mut self) -> Vec<String> {
        self.recording = false;
        self.captured.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, time::Duration};

    use serde_json::json;
    use tempfile::tempdir;

    use super::{
        line_diff, parse_manifest, project_search, project_search_stream,
        project_search_with_options, render_diff, CommandRegistry, DiffOp, DiffRenderMode,
        MacroRecorder, PluginHost, PluginRpcResponse, PluginTransport, SearchOptions,
        SplitDirection, SplitLayout,
    };

    struct FakePluginTransport {
        next_response: PluginRpcResponse,
    }

    impl PluginTransport for FakePluginTransport {
        fn send_json(&mut self, payload: &str) -> crate::core::AppResult<String> {
            let req: super::PluginRpcRequest =
                serde_json::from_str(payload).expect("request should be valid JSON");
            self.next_response.id = req.id;
            serde_json::to_string(&self.next_response)
                .map_err(|err| crate::core::AppError::Validation(err.to_string()))
        }
    }

    #[test]
    fn manifest_parses_and_validates() {
        let raw =
            r#"{"name":"fmt","version":"1.0.0","command":"plugin-fmt","capabilities":["format"]}"#;
        let manifest = parse_manifest(raw).expect("manifest should parse");
        assert_eq!(manifest.name, "fmt");
        assert!(parse_manifest("{}").is_err());
    }

    #[test]
    fn project_search_finds_nested_matches() {
        let dir = tempdir().expect("tempdir");
        let nested = dir.path().join("nested");
        fs::create_dir_all(&nested).expect("create nested");
        fs::write(dir.path().join("a.txt"), "abc\nneedle\n").expect("write file a");
        fs::write(nested.join("b.txt"), "nothing\nneedle\n").expect("write file b");

        let hits = project_search(dir.path(), "needle").expect("search");
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn project_search_supports_filters_and_streaming() {
        let dir = tempdir().expect("tempdir");
        let ignored = dir.path().join("target");
        fs::create_dir_all(&ignored).expect("create excluded dir");
        fs::write(dir.path().join("a.txt"), "needle\nline").expect("write txt");
        fs::write(dir.path().join("b.rs"), "needle\nneedle").expect("write rs");
        fs::write(ignored.join("c.rs"), "needle").expect("write ignored file");

        let options = SearchOptions {
            include_extensions: vec!["rs".into()],
            exclude_dirs: vec!["target".into()],
            max_results: Some(1),
        };
        let hits = project_search_with_options(dir.path(), "needle", &options).expect("search");
        assert_eq!(hits.len(), 1);
        assert!(hits[0].file.ends_with("b.rs"));

        let mut streamed = Vec::new();
        let total = project_search_stream(dir.path(), "needle", &SearchOptions::default(), |hit| {
            streamed.push(hit)
        })
        .expect("stream search");
        assert_eq!(total, 4);
        assert_eq!(streamed.len(), 4);
    }

    #[test]
    fn diff_marks_added_removed_and_unchanged_lines() {
        let diff = line_diff("one\ntwo", "one\nthree");
        assert_eq!(diff[0], DiffOp::Unchanged("one".into()));
        assert_eq!(diff[1], DiffOp::Removed("two".into()));
        assert_eq!(diff[2], DiffOp::Added("three".into()));
    }

    #[test]
    fn diff_renders_in_inline_and_side_by_side_modes() {
        let ops = line_diff("one\ntwo", "one\nthree");
        let inline = render_diff(&ops, DiffRenderMode::Inline);
        assert_eq!(inline[1], (Some("-two".into()), None));

        let side_by_side = render_diff(&ops, DiffRenderMode::SideBySide);
        assert_eq!(side_by_side[1], (Some("two".into()), Some("three".into())));
    }

    #[test]
    fn split_layout_keeps_independent_pane_state() {
        let mut layout = SplitLayout::new(SplitDirection::SideBySide, 0, 1);
        layout.set_cursor(0, 12);
        layout.set_cursor(1, 3);
        layout.set_scroll(0, 80);
        layout.set_scroll(1, 10);
        layout.focus(1);

        assert_eq!(layout.active_pane, 1);
        assert_eq!(layout.panes[0].cursor_offset, 12);
        assert_eq!(layout.panes[1].cursor_offset, 3);
        assert_eq!(layout.panes[0].scroll_offset, 80);
        assert_eq!(layout.panes[1].scroll_offset, 10);
    }

    #[test]
    fn plugin_host_runs_lifecycle_and_rpc_invocation() {
        let manifest = parse_manifest(
            r#"{"name":"fmt","version":"1.0.0","command":"plugin-fmt","capabilities":["format"]}"#,
        )
        .expect("manifest should parse");
        let mut host = PluginHost::new(manifest, Duration::from_secs(3));
        assert_eq!(host.timeout(), Duration::from_secs(3));

        let mut transport = FakePluginTransport {
            next_response: PluginRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: 0,
                result: Some(json!({"formatted":"ok"})),
                error: None,
            },
        };

        assert!(host
            .invoke(&mut transport, "format", json!({"text":"a"}))
            .is_err());
        host.start();
        let result = host
            .invoke(&mut transport, "format", json!({"text":"a"}))
            .expect("rpc call should work");
        assert_eq!(result["formatted"], "ok");
        assert!(host
            .invoke(&mut transport, "validate", json!({"text":"a"}))
            .is_err());
        host.stop();
    }

    #[test]
    fn command_registry_supports_palette_search() {
        let mut reg = CommandRegistry::default();
        reg.register("file.save", "Save current file");
        reg.register("file.open", "Open file");

        let results = reg.search("save");
        assert_eq!(results, vec!["file.save".to_string()]);
    }

    #[test]
    fn macro_recorder_replays_in_inserted_order() {
        let mut macro_recorder = MacroRecorder::default();
        macro_recorder.start();
        macro_recorder.push("open");
        macro_recorder.push("save");
        let replay = macro_recorder.stop();
        assert_eq!(replay, vec!["open".to_string(), "save".to_string()]);
    }
}
