use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::phase0::{AppError, AppResult};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectHit {
    pub file: PathBuf,
    pub line: usize,
    pub text: String,
}

pub fn project_search(root: &Path, needle: &str) -> AppResult<Vec<ProjectHit>> {
    if needle.is_empty() {
        return Ok(Vec::new());
    }

    let mut hits = Vec::new();
    for entry in fs::read_dir(root).map_err(|err| AppError::Validation(err.to_string()))? {
        let entry = entry.map_err(|err| AppError::Validation(err.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            hits.extend(project_search(&path, needle)?);
            continue;
        }

        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };

        for (line_idx, line) in content.lines().enumerate() {
            if line.contains(needle) {
                hits.push(ProjectHit {
                    file: path.clone(),
                    line: line_idx + 1,
                    text: line.to_owned(),
                });
            }
        }
    }

    Ok(hits)
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
    use std::fs;

    use tempfile::tempdir;

    use super::{
        line_diff, parse_manifest, project_search, CommandRegistry, DiffOp, MacroRecorder,
    };

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
    fn diff_marks_added_removed_and_unchanged_lines() {
        let diff = line_diff("one\ntwo", "one\nthree");
        assert_eq!(diff[0], DiffOp::Unchanged("one".into()));
        assert_eq!(diff[1], DiffOp::Removed("two".into()));
        assert_eq!(diff[2], DiffOp::Added("three".into()));
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
