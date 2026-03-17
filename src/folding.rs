use std::collections::{HashMap, HashSet};

/// A foldable region in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldRegion {
    /// 0-based line number where the fold starts (the line with `{` or block opener).
    pub start_line: usize,
    /// 0-based line number where the fold ends (the line with `}` or block closer).
    pub end_line: usize,
}

/// Manages fold state for a document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoldState {
    /// Set of start_line values that are currently collapsed.
    pub collapsed: HashSet<usize>,
    /// Cached fold regions (recomputed when content changes).
    regions: Vec<FoldRegion>,
    /// Hash of the content used to compute regions (for invalidation).
    content_hash: u64,
}

impl FoldState {
    /// Recompute fold regions if content has changed.
    /// `syntax` is the file extension (e.g. "rs", "xml", "json", "py") used to
    /// pick the fold strategy.
    pub fn update_regions(&mut self, content: &str, syntax: &str) {
        let hash = simple_hash(content);
        if hash != self.content_hash {
            self.regions = compute_folds(content, syntax);
            self.content_hash = hash;
            // Remove collapsed entries that no longer exist
            let valid_starts: HashSet<usize> = self.regions.iter().map(|r| r.start_line).collect();
            self.collapsed.retain(|s| valid_starts.contains(s));
        }
    }

    /// Get all fold regions.
    pub fn regions(&self) -> &[FoldRegion] {
        &self.regions
    }

    /// Check if a line is the start of a foldable region.
    pub fn is_fold_start(&self, line: usize) -> bool {
        self.regions.iter().any(|r| r.start_line == line)
    }

    /// Check if a fold starting at `start_line` is currently collapsed.
    pub fn is_collapsed(&self, start_line: usize) -> bool {
        self.collapsed.contains(&start_line)
    }

    /// Toggle fold state for a region starting at `start_line`.
    pub fn toggle(&mut self, start_line: usize) {
        if self.collapsed.contains(&start_line) {
            self.collapsed.remove(&start_line);
        } else if self.is_fold_start(start_line) {
            self.collapsed.insert(start_line);
        }
    }

    /// Collapse all foldable regions.
    pub fn fold_all(&mut self) {
        for r in &self.regions {
            self.collapsed.insert(r.start_line);
        }
    }

    /// Expand all folded regions.
    pub fn unfold_all(&mut self) {
        self.collapsed.clear();
    }

    /// Fold to a specific level (1-8). Level 1 = only top-level folds collapsed.
    /// Level N = collapse all regions whose nesting depth <= N.
    pub fn fold_level(&mut self, level: usize) {
        self.collapsed.clear();
        // Compute nesting depth for each region
        let mut sorted = self.regions.clone();
        sorted.sort_by_key(|r| r.start_line);

        for region in &sorted {
            let depth = self.nesting_depth(region);
            if depth < level {
                self.collapsed.insert(region.start_line);
            }
        }
    }

    /// Compute nesting depth of a region (0 = top level).
    fn nesting_depth(&self, target: &FoldRegion) -> usize {
        self.regions
            .iter()
            .filter(|r| r.start_line < target.start_line && r.end_line > target.end_line)
            .count()
    }

    /// Get the set of lines that should be hidden (inside collapsed folds).
    /// Returns a set of 0-based line numbers to hide.
    pub fn hidden_lines(&self) -> HashSet<usize> {
        let mut hidden = HashSet::new();
        for r in &self.regions {
            if self.collapsed.contains(&r.start_line) {
                // Hide lines from start_line+1 to end_line (inclusive).
                // The start line (with `{`) and end line (with `}`) remain visible.
                // Actually for best UX: show start_line, hide everything from start_line+1 to end_line.
                for line in (r.start_line + 1)..=r.end_line {
                    hidden.insert(line);
                }
            }
        }
        hidden
    }

    /// Build a display string from content, omitting hidden lines.
    /// Returns (display_content, display_line_to_real_line mapping).
    pub fn build_display(&self, content: &str) -> (String, Vec<usize>) {
        if self.collapsed.is_empty() {
            // No folds active — pass through.
            let line_count = content.lines().count().max(1);
            return (content.to_string(), (0..line_count).collect());
        }

        let hidden = self.hidden_lines();
        let mut display = String::with_capacity(content.len());
        let mut line_map = Vec::new(); // display_line_index -> real_line_index

        // Build lookup: start_line -> fold region, for adding fold indicators
        let fold_lookup: HashMap<usize, &FoldRegion> = self
            .regions
            .iter()
            .filter(|r| self.collapsed.contains(&r.start_line))
            .map(|r| (r.start_line, r))
            .collect();

        for (i, line) in content.lines().enumerate() {
            if hidden.contains(&i) {
                continue;
            }
            if !display.is_empty() {
                display.push('\n');
            }
            display.push_str(line);
            // If this is a collapsed fold start, append a fold indicator
            if let Some(region) = fold_lookup.get(&i) {
                let hidden_count = region.end_line - region.start_line;
                display.push_str(&format!(" /* ... {} lines ... */", hidden_count));
            }
            line_map.push(i);
        }

        (display, line_map)
    }

    /// Map a display line number back to the real line number.
    pub fn display_to_real_line(&self, display_line: usize, line_map: &[usize]) -> usize {
        line_map.get(display_line).copied().unwrap_or(display_line)
    }
}

/// Pick the right fold strategy based on file syntax.
fn compute_folds(content: &str, syntax: &str) -> Vec<FoldRegion> {
    // Always include custom fold markers (// {{{ / // }}})
    let mut regions = compute_custom_marker_folds(content);

    let syntax_regions = match syntax {
        // XML/HTML family — fold on matching tags
        "xml" | "html" | "htm" | "svg" | "xaml" | "xsl" | "xslt" | "xsd" | "jsp" | "vue"
        | "svelte" | "rss" | "opml" => compute_xml_folds(content),
        // JSON — fold on braces AND brackets
        "json" | "jsonc" | "geojson" => compute_json_folds(content),
        // Indent-based languages
        "py" | "yaml" | "yml" | "ini" | "cfg" | "conf" => compute_indent_folds(content),
        // Everything else — brace-based (C-family, Rust, Go, Java, C#, PS, etc.)
        _ => compute_brace_folds(content),
    };

    regions.extend(syntax_regions);
    regions.sort_by_key(|r| r.start_line);
    // Deduplicate by start_line
    regions.dedup_by_key(|r| r.start_line);
    regions
}

/// Compute fold regions from custom markers: `// {{{` and `// }}}`.
/// Also supports `# {{{` / `# }}}` for shell/python and `-- {{{` / `-- }}}` for SQL/Lua.
fn compute_custom_marker_folds(content: &str) -> Vec<FoldRegion> {
    let mut regions = Vec::new();
    let mut stack: Vec<usize> = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains("{{{") {
            stack.push(line_idx);
        } else if trimmed.contains("}}}") {
            if let Some(start) = stack.pop() {
                if line_idx > start + 1 {
                    regions.push(FoldRegion {
                        start_line: start,
                        end_line: line_idx,
                    });
                }
            }
        }
    }

    regions
}

/// Compute fold regions based on brace matching `{` `}`.
/// Works for C-family languages (C#, Java, Rust, JS, Go, etc.) and similar.
fn compute_brace_folds(content: &str) -> Vec<FoldRegion> {
    let mut regions = Vec::new();
    let mut stack: Vec<usize> = Vec::new(); // stack of line numbers with open braces

    for (line_idx, line) in content.lines().enumerate() {
        // Count braces on this line (simplified: ignore braces in strings/comments)
        let mut in_string = false;
        let mut in_char = false;
        let mut escape = false;
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];

            if escape {
                escape = false;
                i += 1;
                continue;
            }
            if ch == '\\' && (in_string || in_char) {
                escape = true;
                i += 1;
                continue;
            }

            // Line comment
            if !in_string && !in_char && ch == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                break;
            }

            // String literals
            if ch == '"' && !in_char {
                in_string = !in_string;
            } else if ch == '\'' && !in_string {
                in_char = !in_char;
            }

            if !in_string && !in_char {
                if ch == '{' {
                    stack.push(line_idx);
                } else if ch == '}' {
                    if let Some(open_line) = stack.pop() {
                        // Only create fold if it spans multiple lines
                        if line_idx > open_line + 1 {
                            regions.push(FoldRegion {
                                start_line: open_line,
                                end_line: line_idx,
                            });
                        }
                    }
                }
            }

            i += 1;
        }
    }

    // Sort by start line for consistent ordering
    regions.sort_by_key(|r| r.start_line);
    regions
}

/// Compute fold regions for JSON: braces `{}` and brackets `[]`.
fn compute_json_folds(content: &str) -> Vec<FoldRegion> {
    let mut regions = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut in_string = false;
    let mut escape = false;

    for (line_idx, line) in content.lines().enumerate() {
        for ch in line.chars() {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' && in_string {
                escape = true;
                continue;
            }
            if ch == '"' {
                in_string = !in_string;
                continue;
            }
            if in_string {
                continue;
            }
            if ch == '{' || ch == '[' {
                stack.push(line_idx);
            } else if ch == '}' || ch == ']' {
                if let Some(open_line) = stack.pop() {
                    if line_idx > open_line + 1 {
                        regions.push(FoldRegion {
                            start_line: open_line,
                            end_line: line_idx,
                        });
                    }
                }
            }
        }
    }

    regions.sort_by_key(|r| r.start_line);
    regions
}

/// Compute fold regions for XML/HTML based on matching tags.
fn compute_xml_folds(content: &str) -> Vec<FoldRegion> {
    let mut regions = Vec::new();
    // Stack: (tag_name, start_line)
    let mut stack: Vec<(String, usize)> = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let mut chars = line.char_indices().peekable();
        while let Some((pos, ch)) = chars.next() {
            if ch != '<' {
                continue;
            }
            // Read the rest of the tag on this line
            let tag_start = pos;
            let rest = &line[tag_start..];

            // Find the '>' on this line (tags may span lines but we handle single-line tags)
            let close_pos = match rest.find('>') {
                Some(p) => p,
                None => continue,
            };
            let tag_content = &rest[..=close_pos];

            // Skip comments, CDATA, processing instructions, doctype
            if tag_content.starts_with("<!--")
                || tag_content.starts_with("<![")
                || tag_content.starts_with("<?")
                || tag_content.starts_with("<!")
            {
                continue;
            }

            // Self-closing tag?
            let self_closing = tag_content.ends_with("/>");

            // Closing tag?
            let is_close = tag_content.starts_with("</");

            // Extract tag name
            let name_start = if is_close { 2 } else { 1 };
            let inner = &tag_content[name_start..tag_content.len() - 1]; // strip < and >
            let inner = if self_closing && inner.ends_with('/') {
                &inner[..inner.len() - 1]
            } else {
                inner
            };
            let tag_name: String = inner
                .split(|c: char| c.is_whitespace() || c == '/')
                .next()
                .unwrap_or("")
                .to_lowercase();

            if tag_name.is_empty() {
                continue;
            }

            // Skip void/self-closing HTML elements
            if self_closing {
                continue;
            }

            if is_close {
                // Pop matching open tag from stack
                if let Some(idx) = stack.iter().rposition(|(name, _)| *name == tag_name) {
                    let (_, open_line) = stack.remove(idx);
                    if line_idx > open_line + 1 {
                        regions.push(FoldRegion {
                            start_line: open_line,
                            end_line: line_idx,
                        });
                    }
                }
            } else {
                stack.push((tag_name, line_idx));
            }

            // Advance chars past the tag
            for _ in 0..close_pos {
                chars.next();
            }
        }
    }

    regions.sort_by_key(|r| r.start_line);
    regions
}

/// Compute fold regions based on indentation level changes.
/// Works for Python, YAML, and similar indent-based languages.
fn compute_indent_folds(content: &str) -> Vec<FoldRegion> {
    let mut regions = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return regions;
    }

    // Compute indent level for each line
    let indent_of = |line: &str| -> usize {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            return usize::MAX; // blank lines don't affect fold detection
        }
        line.len() - trimmed.len()
    };

    // Stack: (indent_level, start_line)
    let mut stack: Vec<(usize, usize)> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let indent = indent_of(line);
        if indent == usize::MAX {
            continue; // skip blank lines
        }

        // Close any blocks that have ended (indent decreased)
        while let Some(&(prev_indent, start)) = stack.last() {
            if indent <= prev_indent {
                stack.pop();
                // Find the last non-blank line before this one at the higher indent
                let mut end = i.saturating_sub(1);
                while end > start && indent_of(lines[end]) == usize::MAX {
                    end = end.saturating_sub(1);
                }
                if end > start + 1 {
                    regions.push(FoldRegion {
                        start_line: start,
                        end_line: end,
                    });
                }
            } else {
                break;
            }
        }

        // If the next non-blank line has a greater indent, this starts a new block
        let mut next = i + 1;
        while next < lines.len() && indent_of(lines[next]) == usize::MAX {
            next += 1;
        }
        if next < lines.len() && indent_of(lines[next]) > indent {
            stack.push((indent, i));
        }
    }

    // Close remaining open blocks at end of file
    let total = lines.len();
    while let Some((_, start)) = stack.pop() {
        let mut end = total.saturating_sub(1);
        while end > start && indent_of(lines[end]) == usize::MAX {
            end = end.saturating_sub(1);
        }
        if end > start + 1 {
            regions.push(FoldRegion {
                start_line: start,
                end_line: end,
            });
        }
    }

    regions.sort_by_key(|r| r.start_line);
    regions
}

/// Simple hash for content change detection.
fn simple_hash(s: &str) -> u64 {
    let mut h: u64 = 0;
    for b in s.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as u64);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_brace_folds() {
        let content = "fn main() {\n    let x = 1;\n    let y = 2;\n}\n";
        let regions = compute_brace_folds(content);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].start_line, 0);
        assert_eq!(regions[0].end_line, 3);
    }

    #[test]
    fn detects_nested_folds() {
        let content = "class Foo {\n    void bar() {\n        x();\n    }\n}\n";
        let regions = compute_brace_folds(content);
        assert_eq!(regions.len(), 2);
        // Inner fold
        assert!(regions.iter().any(|r| r.start_line == 1 && r.end_line == 3));
        // Outer fold
        assert!(regions.iter().any(|r| r.start_line == 0 && r.end_line == 4));
    }

    #[test]
    fn ignores_single_line_braces() {
        let content = "if (x) { y(); }\n";
        let regions = compute_brace_folds(content);
        assert_eq!(regions.len(), 0);
    }

    #[test]
    fn ignores_braces_in_strings() {
        let content = "let s = \"{ not a fold }\";\nlet t = 1;\n";
        let regions = compute_brace_folds(content);
        assert_eq!(regions.len(), 0);
    }

    #[test]
    fn ignores_braces_in_comments() {
        let content = "// { not a fold }\nlet t = 1;\n";
        let regions = compute_brace_folds(content);
        assert_eq!(regions.len(), 0);
    }

    #[test]
    fn fold_toggle_and_collapse() {
        let content = "fn main() {\n    x();\n    y();\n}\n";
        let mut state = FoldState::default();
        state.update_regions(content, "rs");

        assert!(!state.is_collapsed(0));
        state.toggle(0);
        assert!(state.is_collapsed(0));
        state.toggle(0);
        assert!(!state.is_collapsed(0));
    }

    #[test]
    fn hidden_lines_correct() {
        let content = "fn main() {\n    x();\n    y();\n}\nend\n";
        let mut state = FoldState::default();
        state.update_regions(content, "rs");
        state.toggle(0); // collapse main

        let hidden = state.hidden_lines();
        assert!(hidden.contains(&1)); // x();
        assert!(hidden.contains(&2)); // y();
        assert!(hidden.contains(&3)); // }
        assert!(!hidden.contains(&0)); // fn main() { — stays visible
        assert!(!hidden.contains(&4)); // end — stays visible
    }

    #[test]
    fn build_display_omits_folded_lines() {
        let content = "fn main() {\n    x();\n    y();\n}\nend";
        let mut state = FoldState::default();
        state.update_regions(content, "rs");
        state.toggle(0);

        let (display, line_map) = state.build_display(content);
        // Should show: "fn main() { /* ... 3 lines ... */\nend"
        assert_eq!(line_map.len(), 2); // 2 visible lines
        assert_eq!(line_map[0], 0); // display line 0 = real line 0
        assert_eq!(line_map[1], 4); // display line 1 = real line 4
        assert!(display.contains("... 3 lines ..."));
        assert!(display.contains("end"));
    }

    #[test]
    fn fold_all_and_unfold_all() {
        let content = "a {\n  b\n}\nc {\n  d\n}\n";
        let mut state = FoldState::default();
        state.update_regions(content, "rs");

        state.fold_all();
        assert_eq!(state.collapsed.len(), 2);

        state.unfold_all();
        assert!(state.collapsed.is_empty());
    }

    // -- JSON fold tests --

    #[test]
    fn json_folds_braces_and_brackets() {
        let content = "{\n  \"a\": [\n    1,\n    2\n  ]\n}";
        let regions = compute_json_folds(content);
        assert!(regions.len() >= 2);
        // Outer object fold
        assert!(regions.iter().any(|r| r.start_line == 0 && r.end_line == 5));
        // Inner array fold
        assert!(regions.iter().any(|r| r.start_line == 1 && r.end_line == 4));
    }

    #[test]
    fn json_ignores_braces_in_strings() {
        let content = "{\n  \"key\": \"{ not a fold }\"\n}";
        let regions = compute_json_folds(content);
        assert_eq!(regions.len(), 1); // only the outer {}
    }

    // -- XML fold tests --

    #[test]
    fn xml_folds_matching_tags() {
        let content = "<root>\n  <child>\n    text\n  </child>\n</root>";
        let regions = compute_xml_folds(content);
        assert!(regions.len() >= 2);
        assert!(regions.iter().any(|r| r.start_line == 0 && r.end_line == 4));
        assert!(regions.iter().any(|r| r.start_line == 1 && r.end_line == 3));
    }

    #[test]
    fn xml_ignores_self_closing() {
        let content = "<root>\n  <br/>\n  <img src=\"x\"/>\n</root>";
        let regions = compute_xml_folds(content);
        assert_eq!(regions.len(), 1); // only <root>...</root>
    }

    // -- Indent fold tests --

    #[test]
    fn indent_folds_python() {
        let content = "def foo():\n    x = 1\n    y = 2\n\nz = 3";
        let regions = compute_indent_folds(content);
        assert!(!regions.is_empty());
        assert!(regions.iter().any(|r| r.start_line == 0));
    }

    #[test]
    fn indent_folds_yaml() {
        let content = "root:\n  child1: a\n  child2: b\nother: c";
        let regions = compute_indent_folds(content);
        assert!(!regions.is_empty());
        assert!(regions.iter().any(|r| r.start_line == 0));
    }

    // -- Custom marker tests --

    #[test]
    fn custom_marker_folds() {
        let content = "// {{{ Section\nline1\nline2\n// }}}\n";
        let regions = compute_custom_marker_folds(content);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].start_line, 0);
        assert_eq!(regions[0].end_line, 3);
    }

    #[test]
    fn fold_level_collapses_top_level() {
        // Nested: outer { inner { ... } }
        let content = "outer {\n  inner {\n    x\n  }\n}\n";
        let mut state = FoldState::default();
        state.update_regions(content, "rs");
        state.fold_level(1); // collapse only top-level
        assert!(state.is_collapsed(0)); // outer is top-level (depth 0 < 1)
        assert!(!state.is_collapsed(1)); // inner is depth 1, not < 1
    }

    // -- Dispatch test --

    #[test]
    fn dispatch_picks_right_strategy() {
        let json = "{\n  \"a\": 1\n}";
        let regions = compute_folds(json, "json");
        assert!(!regions.is_empty());

        let xml = "<r>\n  <c>x</c>\n</r>";
        let regions = compute_folds(xml, "xml");
        assert!(!regions.is_empty());

        let py = "def f():\n    pass\n    pass\nx = 1";
        let regions = compute_folds(py, "py");
        assert!(!regions.is_empty());

        let rs = "fn f() {\n    x();\n    y();\n}";
        let regions = compute_folds(rs, "rs");
        assert!(!regions.is_empty());
    }
}
