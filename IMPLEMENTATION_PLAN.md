# CodeEdit — Implementation Plan

**Source**: `notepad-plus-plus-clone-prd.md` v1.0
**Tech Stack**: Rust + egui 0.28 / eframe 0.28 + syntect
**Last synced**: 2026-03-16

> Checkboxes sync with implementation status. Checked = implemented and tested.

---

## Technology & Library Decisions

| Concern | Library | Version | Rationale |
|---------|---------|---------|-----------|
| UI Framework | `egui` / `eframe` | 0.28 | Immediate-mode, cross-platform, already in use |
| Syntax Highlighting | `syntect` (via `egui_extras`) | latest | TextMate grammars, 50+ languages bundled |
| Text Buffer | `ropey` | 1.x | O(log n) edits, handles multi-GB files, line indexing |
| Regex Search | `regex` | 1.x | PCRE-like, fast, safe |
| File Dialogs | `rfd` | 0.14 | Native OS dialogs, already in use |
| Serialization | `serde` + `serde_json` | 1.0 | Already in use for session/settings |
| XML | `quick-xml` | 0.36 | Already in use for validation |
| Encoding | `encoding_rs` | 0.8 | Detect/convert UTF-8, UTF-16 LE/BE, ANSI, BOM |
| Diff Engine | `similar` | 2.x | Production Myers diff, replaces hand-rolled line diff |
| File Watching | `notify` | 7.x | Cross-platform fs events, replaces 1s polling |
| CLI Parsing | `clap` | 4.x | Args: `codeedit [FILES...] -n LINE` |
| Hashing | `sha2` + `md-5` + `base64` | latest | TU-06/07 hash and encode utilities |
| Plugin IPC | JSON-RPC 2.0 over stdio | — | Already designed in extensibility.rs |
| Benchmarks | `criterion` | 0.5 | Repeatable micro-benchmarks with regression detection |
| Coverage | `cargo-llvm-cov` | latest | Line + branch coverage gating |

---

## Testing Strategy for Desktop App

| Layer | Tool / Technique | What It Covers |
|-------|------------------|----------------|
| **Unit tests** | `#[cfg(test)]` + `cargo test` | All business logic, models, services, algorithms |
| **Integration tests** | `tests/*.rs` + `tempfile` | File I/O roundtrips, multi-module workflows, session persistence |
| **Headless UI tests** | `egui::Context::run()` in tests | UI state transitions, panel visibility, widget interactions — no window needed |
| **Accessibility E2E** | `accesskit` queries within egui + Windows UI Automation (`windows` crate `UIAutomation`) | Launch real process, find elements by role/name, click, type, assert text |
| **Screenshot regression** | `eframe` render-to-image + `image` crate pixel diff | Visual regression for themes, layout, syntax highlighting |
| **Fuzzing** | `cargo-fuzz` / `arbtest` | Large file handling, encoding edge cases, malformed input |
| **Benchmarks** | `criterion` + custom harness | File open time, search perf, render latency, memory usage |

### How headless egui tests work

```rust
#[test]
fn test_find_panel_toggles() {
    let ctx = egui::Context::default();
    let mut app = RustNotepadApp::default();
    // simulate Ctrl+F
    ctx.run(egui::RawInput::default(), |ctx| {
        app.show_find_panel = true;
        app.update(ctx, &mut eframe::Frame::default()); // or just call UI methods
    });
    assert!(app.find_state.show_panel);
}
```

### How Windows UI Automation E2E tests work

```rust
// tests/e2e_windows.rs  (only compiled on Windows)
#[test]
#[cfg(target_os = "windows")]
fn test_open_file_via_menu() {
    let child = std::process::Command::new("target/debug/rust-notepad.exe").spawn().unwrap();
    std::thread::sleep(Duration::from_secs(2));
    // Use windows crate UIAutomation COM APIs to find window, invoke menu, etc.
    // Assert tab bar contains expected filename
    child.kill().unwrap();
}
```

---

## Current Implementation Audit

### What Exists (Backend Primitives)

| Module | File | What's Built |
|--------|------|--------------|
| Core models | `src/core.rs` | `Document`, `TabId`, `SessionState`, `SearchQuery`, `Diagnostic`, `AppError`, `Clock` |
| Tab/File ops | `src/editor_state.rs` | `EditorState`: new/close/close-all/close-others tabs, load/save file, find/replace, external change detection |
| Services | `src/editor_services.rs` | Find-in-open-tabs, `RecentFiles` LRU, session serialize/deserialize, JSON/XML validation, `Settings`/`Keybinding` |
| Extensibility | `src/extensibility.rs` | Plugin manifest/RPC, project search (with filters, streaming), line diff, `SplitLayout`, `CommandRegistry`, `MacroRecorder` |
| UI App | `src/app.rs` | egui app scaffold: menu bar, tab bar, text editor (egui `TextEdit`), find panel (side), status bar (syntax only), 2 example plugins |

### Test Inventory

| Location | Count | Type |
|----------|-------|------|
| `src/core.rs` | ~9 | Unit |
| `src/editor_state.rs` | ~8 | Unit + integration (tempfile) |
| `src/editor_services.rs` | ~7 | Unit |
| `src/extensibility.rs` | ~12 | Unit |
| `src/app.rs` | 1 | Unit (word boundary) |
| `tests/extensibility_e2e.rs` | 1 | Integration E2E |
| **Total** | **~38** | |

### Gap Summary vs PRD

| Category | Requirements | Done | Partial | Gap |
|----------|-------------|------|---------|-----|
| Core Editing (CE) | 23 | 0 | 3 | 20 |
| File & Session (FM) | 12 | 2 | 3 | 7 |
| Syntax Highlighting (SH) | 13 | 1 | 3 | 9 |
| Search & Replace (SR) | 14 | 1 | 3 | 10 |
| Code Folding (CF) | 9 | 0 | 0 | 9 |
| MDI / Tabs (MDI) | 11 | 2 | 0 | 9 |
| Macros (MA) | 9 | 0 | 1 | 8 |
| Plugins (PL) | 8 | 0 | 2 | 6 |
| Customization (CU) | 10 | 0 | 2 | 8 |
| Tools (TU) | 13 | 0 | 1 | 12 |
| **Totals** | **122** | **6** | **18** | **98** |

**The existing code provides a solid architectural skeleton (~20% of the work) but nearly all PRD P0 features still need implementation, especially the editor engine and UI.**

---

## Phase 1 — MVP Core Editor (All P0 Requirements)

**Goal**: A usable daily-driver editor for developers on Windows, covering all 🔴 P0 items.
**Timeline**: Months 1–3

---

### 1.1 Text Buffer Engine

**PRD**: CE-01 (4 GB files), CE-13 (undo/redo)

| Task | Status |
|------|--------|
| - [ ] Add `ropey` crate to `Cargo.toml` | GAP |
| - [ ] Create `src/text_buffer.rs` with `TextBuffer` wrapping `ropey::Rope` | GAP |
| - [ ] API: `insert(pos, text)`, `delete(range)`, `slice(range)`, `to_string()` | GAP |
| - [ ] Line-indexed access: `line(n)`, `line_count()`, `char_to_line()`, `line_to_char()` | GAP |
| - [ ] Implement `UndoManager` with operation-based undo/redo stack | GAP |
| - [ ] `UndoOp` enum: `Insert { pos, len }`, `Delete { pos, text }` | GAP |
| - [ ] Coalesce rapid keystrokes into single undo groups | GAP |
| - [ ] Migrate `Document.content` from `String` to `TextBuffer` | GAP |
| - [ ] Benchmark: open 100 MB file in < 2 s | GAP |

**Deliverables**: `src/text_buffer.rs` (TextBuffer + UndoManager)

**Unit tests** (100% coverage):
- [ ] Insert/delete at start, middle, end of buffer
- [ ] Line indexing: `char_to_line` / `line_to_char` roundtrip
- [ ] Undo single op, undo coalesced group, redo after undo
- [ ] Undo stack clears redo on new edit
- [ ] Empty buffer edge cases (delete from empty, undo empty stack)
- [ ] Large file: load 100 MB → verify line count

**Integration test**:
- [ ] Open 100 MB file → insert text at line 500,000 → save → verify content correct

---

### 1.2 Encoding & EOL

**PRD**: CE-10 (EOL conversion), CE-11 (encoding), FM-12 (save with different encoding)

| Task | Status |
|------|--------|
| - [ ] Add `encoding_rs` crate to `Cargo.toml` | GAP |
| - [ ] Create `src/encoding.rs` | GAP |
| - [ ] `EolStyle` enum: `CRLF`, `LF`, `CR` | GAP |
| - [ ] `EncodingInfo { encoding, has_bom, eol }` stored on each `Document` | GAP |
| - [ ] `detect_encoding(bytes: &[u8]) → EncodingInfo` — BOM sniff + heuristic | GAP |
| - [ ] `decode_file(bytes, encoding) → String` | GAP |
| - [ ] `encode_file(text, encoding, eol) → Vec<u8>` | GAP |
| - [ ] UI: encoding + EOL indicators in status bar | GAP |
| - [ ] UI: status bar click → dropdown to change encoding/EOL | GAP |
| - [ ] EOL conversion on save | GAP |

**Deliverables**: `src/encoding.rs`, status bar encoding/EOL display

**Unit tests** (100% coverage):
- [ ] Detect UTF-8 with BOM, UTF-8 without BOM
- [ ] Detect UTF-16 LE BOM, UTF-16 BE BOM
- [ ] Detect ANSI (fallback)
- [ ] Decode/encode roundtrip: UTF-8, UTF-16 LE, UTF-16 BE
- [ ] EOL detection: pure CRLF, pure LF, pure CR, mixed → majority wins
- [ ] EOL conversion: LF → CRLF, CRLF → LF, CR → LF

**Integration test**:
- [ ] Write UTF-16 LE file with BOM + CRLF → open → verify decoded → save as UTF-8 LF → verify bytes on disk

---

### 1.3 Custom Code Editor Widget

**PRD**: CE-02 (multi-caret), CE-03 (column selection), CE-07 (word wrap), CE-08 (line numbers), CE-18 (zoom), CE-23 (overtype)

The built-in `egui::TextEdit` cannot support multi-caret, column selection, folding, or custom gutter rendering. A custom widget is required.

| Task | Status |
|------|--------|
| - [ ] Create `src/code_editor.rs` — custom egui widget replacing `TextEdit::multiline` | GAP |
| - [ ] Text layout engine: lay out lines from `TextBuffer`, apply word wrap | GAP |
| - [ ] `CursorSet` struct: `Vec<CursorRange>` with primary cursor | GAP |
| - [ ] Cursor movement: arrow keys, Home/End, Ctrl+Left/Right (word), Page Up/Down | GAP |
| - [ ] Selection: Shift+arrow, Shift+Ctrl+arrow, Shift+Home/End | GAP |
| - [ ] Multi-caret: Alt+Click to add cursor, Ctrl+Alt+Up/Down for adjacent lines | GAP |
| - [ ] Column/block selection: Alt+Drag | GAP |
| - [ ] Multi-caret typing: insert/delete at all cursors simultaneously | GAP |
| - [ ] Word wrap modes: None, WindowEdge, Column(n) | GAP |
| - [ ] Line number gutter (togglable via View menu) | GAP (partial in egui) |
| - [ ] Click gutter → select entire line | GAP |
| - [ ] Zoom: Ctrl+Scroll or Ctrl+Plus/Minus, per-document font size | GAP |
| - [ ] Overtype mode: Insert key toggle, replace char under cursor | GAP |
| - [ ] Status bar: INS/OVR indicator | GAP |
| - [ ] Merge overlapping cursors automatically | GAP |
| - [ ] Integrate syntect highlighting into custom widget | GAP |

**Deliverables**: `src/code_editor.rs`, `src/cursor.rs`

**Unit tests** (100% coverage):
- [ ] CursorSet: add cursor, merge overlapping, sort order
- [ ] Multi-caret insert: 3 cursors → type "x" → verify 3 "x" inserted at correct positions
- [ ] Column selection: Alt+Drag from (0,0) to (5,3) → verify rectangular selection
- [ ] Word wrap: line of 100 chars at column 80 → verify 2 visual lines
- [ ] Gutter width: 1-digit (< 10 lines) vs 5-digit (100K+ lines) auto-sizing
- [ ] Zoom clamp: min 6pt, max 72pt
- [ ] Overtype: type "abc" over "xyz" → result "abc" not "abcxyz"

**Headless UI test**:
- [ ] Create widget with text → simulate Alt+Click at 2 positions → type → verify multi-caret result
- [ ] Toggle overtype → type → verify overwrite behavior

---

### 1.4 Auto-Indent & Brace Matching

**PRD**: CE-04 (auto-indent), CE-06 (brace matching/auto-close)

| Task | Status |
|------|--------|
| - [ ] `auto_indent_on_newline(buffer, cursor)` — copy leading whitespace from current line | GAP |
| - [ ] Brace auto-close: typing `{` inserts `{}` with cursor between | GAP |
| - [ ] Pairs: `()`, `[]`, `{}`, `""`, `''`, `` ` ` `` | GAP |
| - [ ] Skip closing char if already present after cursor | GAP |
| - [ ] Brace matching highlight: cursor on `{` highlights matching `}` | GAP |
| - [ ] Stack-based matching algorithm for nested braces | GAP |

**Deliverables**: `src/indent.rs`, `src/brace_match.rs`

**Unit tests** (100% coverage):
- [ ] Auto-indent: newline after `    foo` → next line starts with `    `
- [ ] Auto-indent with tabs vs spaces
- [ ] Auto-close each pair type: `(`, `[`, `{`, `"`, `'`
- [ ] Skip close: cursor before `)` → type `)` → moves past, doesn't double
- [ ] Brace match: nested `{{}}`  → cursor on outer `{` → match is outer `}`
- [ ] Unmatched brace → returns None
- [ ] Mixed brace types: `({})` → correct matching

**Headless UI test**:
- [ ] Type `{` → verify auto-close inserted `}`

---

### 1.5 Whitespace Visualization & Tab Settings

**PRD**: CE-09 (whitespace/EOL viz), CE-20 (tab-to-spaces), CU-09 (tab size)

| Task | Status |
|------|--------|
| - [ ] Render whitespace symbols: `·` for space, `→` for tab, `¶` for newline | GAP |
| - [ ] Toggle via View menu: Show Whitespace | GAP |
| - [ ] Tab size setting: 2, 4, 8 (global default + per-language override) | GAP |
| - [ ] Spaces-on-Tab: Insert spaces when Tab key pressed (configurable) | GAP |
| - [ ] Tab-to-spaces / spaces-to-tabs conversion commands (Edit menu) | GAP |

**Deliverables**: Whitespace rendering in CodeEditor, `src/settings.rs` tab settings

**Unit tests** (100% coverage):
- [ ] Tab-to-space: `\t` with tab_size=4 → `    ` (4 spaces)
- [ ] Spaces-to-tab: `        ` (8 spaces, tab_size=4) → `\t\t`
- [ ] Per-language lookup: Python defaults to 4, Go defaults to tab

---

### 1.6 File & Session Management

**PRD**: FM-01 (CRUD), FM-02 (recent files), FM-03 (external change), FM-06 (session), FM-09 (CLI), FM-11 (drag-drop), FM-12 (save encoding)

| Task | Status |
|------|--------|
| - [x] New / Open / Save / Save As / Close / Close All | DONE |
| - [ ] Save All command (iterate all dirty docs) | GAP |
| - [x] External change detection (polling, 1s) | DONE |
| - [ ] Replace polling with `notify` file watcher | GAP |
| - [ ] Recent files: UI menu with last 10 files, clickable | GAP (backend LRU exists) |
| - [ ] Persist recent files to `~/.codeedit/recent.json` | GAP |
| - [ ] Session save on exit: all tab paths + cursor positions + scroll | GAP (serialize exists) |
| - [ ] Session restore on launch: reopen from `~/.codeedit/session.json` | GAP |
| - [ ] Dirty-close prompt: "Save changes to X?" Yes/No/Cancel | GAP |
| - [ ] Add `clap` crate for CLI arg parsing | GAP |
| - [ ] CLI: `codeedit file1.rs file2.py` — open files on launch | GAP |
| - [ ] CLI: `codeedit -n 50 file.rs` — open at line 50 | GAP |
| - [ ] Drag-and-drop files from OS file manager into window | GAP |

**Deliverables**: `src/session.rs`, CLI in `src/main.rs`, updated `src/app.rs`

**Unit tests** (100% coverage):
- [ ] Session serialize → deserialize roundtrip with cursor positions
- [ ] Recent files: visit 15 files with capacity 10 → verify oldest evicted
- [ ] Recent files: revisit existing → moves to front
- [ ] CLI parsing: `["-n", "50", "file.rs"]` → line=50, file="file.rs"

**Integration tests**:
- [ ] Save session JSON → new EditorState → restore → verify all tabs + positions
- [ ] Write file externally → notify triggers reload prompt

---

### 1.7 Syntax Highlighting — 50+ Languages

**PRD**: SH-01 (50+ langs), SH-02 (priority langs), SH-03 (auto-detect), SH-04 (manual override), SH-07 (style configurator), SH-08 (global theme), SH-11 (highlight occurrences), SH-12 (current line)

| Task | Status |
|------|--------|
| - [x] Language auto-detection by file extension | DONE |
| - [ ] Expand syntax map: load all syntect bundled definitions (~50 languages) | GAP (16 mapped) |
| - [ ] First-line heuristic: shebang `#!/usr/bin/env python` → Python | GAP |
| - [ ] Manual language override: dropdown in status bar | GAP |
| - [ ] Current line highlight: subtle background color on line with cursor | GAP |
| - [ ] Highlight all occurrences of selected word (double-click triggers) | GAP |
| - [ ] Global theme: Light + Dark built-in, applied to all syntax classes | GAP (dark only, hardcoded) |
| - [ ] Style Configurator: color/font/bold per token class | GAP |

**Deliverables**: Updated syntax map, `src/themes.rs`, language selector, occurrence highlight

**Unit tests** (100% coverage):
- [ ] Extension mapping for all priority languages (C, C++, Python, JS, TS, Rust, Go, HTML, CSS, JSON, YAML, etc.)
- [ ] Shebang detection: `#!/usr/bin/env python3` → Python, `#!/bin/bash` → Bash
- [ ] Occurrence count: "foo bar foo" with "foo" selected → 2 occurrences
- [ ] Theme token resolution: keyword → color, string → color, comment → color

---

### 1.8 Search & Replace — Full P0

**PRD**: SR-01 (find bar), SR-02 (find & replace), SR-03 (regex), SR-04 (case/word), SR-05 (wrap-around), SR-06 (find in files), SR-09 (results panel), SR-14 (go to line)

| Task | Status |
|------|--------|
| - [x] Case-sensitive and whole-word options | DONE |
| - [ ] Add `regex` crate to `Cargo.toml` | GAP |
| - [ ] Refactor find UI: inline find bar at top of editor (Ctrl+F) | GAP (side panel) |
| - [ ] Find & Replace dialog (Ctrl+H): forward, backward, replace one/all | GAP (basic exists) |
| - [ ] Search modes: Normal, Extended (`\n` `\t` `\0`), Regex (PCRE) | GAP |
| - [ ] Wrap-around search: continue from top after reaching bottom | GAP |
| - [ ] Find in Files: directory path + glob → results in bottom panel | GAP (backend exists) |
| - [ ] Search results panel: tree of File → Line → Context, clickable | GAP |
| - [ ] Highlight all matches in editor, current match in distinct color | GAP |
| - [ ] Go to Line dialog (Ctrl+G): input number, jump | GAP |
| - [ ] Keyboard shortcuts: F3 find next, Shift+F3 find previous | GAP |

**Deliverables**: `src/search.rs` (expanded), find bar widget, results panel, Go to Line dialog

**Unit tests** (100% coverage):
- [ ] Regex: `log.*Error` finds "logSomeError", "log_Error"
- [ ] Regex: special chars `\.` matches literal dot
- [ ] Extended: `hello\nworld` matches across lines
- [ ] Wrap-around: cursor at end, search wraps to match at start
- [ ] Find in Files: 10 files in temp dir, 3 contain match → 3 results
- [ ] Go to Line: line 0 clamps to 1, line > max clamps to max

**Integration test**:
- [ ] Create temp dir with 10 source files → Find in Files "TODO" → verify all matches found with correct file:line

**Headless UI test**:
- [ ] Ctrl+G → type "50" → Enter → verify cursor at line 50

---

### 1.9 Code Folding

**PRD**: CF-01 (syntax folding), CF-02 (indent folding), CF-03 (fold/unfold all)

| Task | Status |
|------|--------|
| - [ ] Create `src/folding.rs` with `FoldingEngine` | GAP |
| - [ ] Syntax-based folding: detect `{`/`}` pairs (C-like) | GAP |
| - [ ] Indent-based folding: detect indent level changes (Python, YAML) | GAP |
| - [ ] `FoldRegion { start_line, end_line, collapsed: bool }` | GAP |
| - [ ] Fold gutter markers: ▶ (collapsed) / ▼ (expanded) | GAP |
| - [ ] Click fold marker to toggle | GAP |
| - [ ] View menu: Fold All, Unfold All, Fold Level 1–8 | GAP |
| - [ ] Collapsed region: show `...` placeholder, skip lines in rendering | GAP |

**Deliverables**: `src/folding.rs`, fold gutter in CodeEditor

**Unit tests** (100% coverage):
- [ ] Brace fold: `fn main() {\n  ...\n}` → fold region lines 0–2
- [ ] Nested fold: outer and inner detected separately
- [ ] Indent fold: Python `def` with 4-space body → fold at indent decrease
- [ ] Fold state: collapse → lines hidden, unfold → lines visible
- [ ] Fold All → all regions collapsed, Unfold All → all expanded
- [ ] Fold Level 2: only level-1 and level-2 regions collapsed

---

### 1.10 Tab Bar Enhancements

**PRD**: MDI-01 (scrollable tabs), MDI-02 (drag reorder), MDI-03 (context menu), MDI-04 (modified indicator)

| Task | Status |
|------|--------|
| - [x] Basic tab bar with document titles | DONE |
| - [x] Modified indicator (`*` on dirty tabs) | DONE |
| - [ ] Scrollable tab bar when tabs overflow window width | GAP |
| - [ ] Tab drag-and-drop reordering | GAP |
| - [ ] Tab context menu (right-click): Close, Close Others, Close to Right, Copy Full Path | GAP |
| - [ ] Change dirty indicator from `*` to `●` dot | GAP |

**Deliverables**: Enhanced tab bar in `src/app.rs`

**Unit tests** (100% coverage):
- [ ] Tab reorder: move tab 3 to position 1 → verify order
- [ ] Close to Right: tabs [A, B, C, D], active=B → close C, D
- [ ] Copy path: returns `Document.path` as string

---

### 1.11 Customization & Preferences

**PRD**: CU-01 (keyboard remap), CU-03 (hide/show), CU-04 (per-lang font), CU-05 (global font), CU-06 (preferences dialog), CU-09 (tab size)

| Task | Status |
|------|--------|
| - [ ] Create `src/preferences_dialog.rs` | GAP |
| - [ ] Preferences dialog with categorized panels: General, Editor, Keyboard | GAP |
| - [ ] General panel: theme (light/dark), default font family, default font size | GAP |
| - [ ] Editor panel: tab size, spaces vs tabs, word wrap mode, show whitespace | GAP |
| - [ ] Keyboard panel: shortcut remapping table (command → chord) | GAP (data model exists) |
| - [ ] Persist settings to `~/.codeedit/settings.json` | GAP |
| - [ ] View menu: toggle toolbar, status bar, tab bar, line numbers | GAP |
| - [ ] Per-language font override (e.g., Markdown uses serif) | GAP |

**Deliverables**: `src/preferences_dialog.rs`, expanded `src/settings.rs`

**Unit tests** (100% coverage):
- [ ] Settings serialize/deserialize roundtrip
- [ ] Keybinding conflict: two commands with same chord → detected
- [ ] Per-language setting resolution: specific > global default
- [ ] Theme switching: "light" → "dark" → verify color changes

**Integration test**:
- [ ] Write settings.json → launch app → verify settings applied

---

### 1.12 Status Bar & Basic Utilities

**PRD**: TU-04 (trim whitespace), TU-12 (word count), TU-13 (status bar)

| Task | Status |
|------|--------|
| - [ ] Full status bar: Ln X, Col Y, Total Lines, Sel N chars, Encoding, EOL, Syntax, INS/OVR, File Size | GAP (syntax only) |
| - [ ] Word count / char count / line count (updated on change) | GAP |
| - [ ] Trim trailing whitespace command (Edit menu) | GAP |

**Deliverables**: Updated status bar in `src/app.rs`

**Unit tests** (100% coverage):
- [ ] Word count: "hello world\nfoo" → 3 words, 15 chars, 2 lines
- [ ] Trim whitespace: `"hello   \nworld  \n"` → `"hello\nworld\n"`
- [ ] Status info computation: line/col from cursor position

---

### Phase 1 — Concrete Deliverables Summary

| New File | Purpose |
|----------|---------|
| `src/text_buffer.rs` | Rope-based buffer + UndoManager |
| `src/encoding.rs` | Encoding detection, decode, encode, EOL |
| `src/code_editor.rs` | Custom egui widget (multi-caret, gutter, wrap, fold gutter) |
| `src/cursor.rs` | CursorSet, CursorRange, column selection geometry |
| `src/indent.rs` | Auto-indent, tab/space conversion |
| `src/brace_match.rs` | Brace matching + auto-close |
| `src/folding.rs` | Fold regions, syntax/indent strategies |
| `src/search.rs` | Regex search, extended mode, wrap-around |
| `src/themes.rs` | Theme definitions, light/dark, style configurator data |
| `src/session.rs` | Session persistence to disk |
| `src/preferences_dialog.rs` | Preferences UI |

| Modified File | Changes |
|---------------|---------|
| `Cargo.toml` | Add ropey, encoding_rs, regex, notify, clap |
| `src/core.rs` | Document uses TextBuffer, adds EncodingInfo |
| `src/app.rs` | Replace TextEdit with CodeEditor, new panels, status bar |
| `src/main.rs` | CLI arg parsing with clap |
| `src/editor_state.rs` | Use TextBuffer for load/save |
| `src/settings.rs` | Expanded settings model |

### Phase 1 — Integration & E2E Tests

- [ ] **Full workflow**: Launch → open file → edit → find/replace → save → close → session restore → reopen
- [ ] **Multi-file search**: Open 20 files → Find in Files → navigate results → close all
- [ ] **Large file**: Open 100 MB file → scroll → search → edit → save (< 2s open)
- [ ] **Encoding roundtrip**: Open UTF-16 LE → convert to UTF-8 → save → verify
- [ ] **Accessibility E2E** (Windows UI Automation):
  - [ ] Find main window → navigate menu (File → Open) → verify tab appears
  - [ ] Type text → Ctrl+F → search → verify results
  - [ ] Full keyboard navigation (Tab through all elements)

---

## Phase 2 — Feature Complete (All P1 Requirements)

**Goal**: Feature parity with Notepad++ for majority of use cases.
**Timeline**: Months 4–6

---

### 2.1 Smart Indent (CE-05)

| Task | Status |
|------|--------|
| - [ ] Language-aware indent: increase after `{`, `[`, `(`, `:` | GAP |
| - [ ] Decrease indent on typing `}`, `]`, `)` | GAP |
| - [ ] Language indent config files | GAP |

**Deliverables**: `src/smart_indent.rs`

**Unit tests**: indent increase/decrease for C, Python, JS, Rust
**Headless UI test**: Type `{` + Enter → verify increased indent

---

### 2.2 Auto-Completion from Document (CE-15)

| Task | Status |
|------|--------|
| - [ ] Extract all words from current document | GAP |
| - [ ] Ctrl+Space → popup with filtered word list | GAP |
| - [ ] Tab/Enter to accept, Esc to dismiss | GAP |
| - [ ] Update word list on document change | GAP |

**Deliverables**: `src/autocomplete.rs`, popup widget

**Unit tests**: word extraction, prefix filtering, deduplication
**Headless UI test**: Type partial word → Ctrl+Space → verify popup contents

---

### 2.3 Drag-and-Drop Text (CE-19)

| Task | Status |
|------|--------|
| - [ ] Select text → drag to new position (same document) | GAP |
| - [ ] Drag text between documents (cross-tab) | GAP |

**Deliverables**: DnD in CodeEditor

**Unit tests**: text move logic, cursor adjustment after move

---

### 2.4 Bookmarks (CE-21)

| Task | Status |
|------|--------|
| - [ ] Toggle bookmark: Ctrl+F2 on current line | GAP |
| - [ ] Navigate: F2 next, Shift+F2 previous (wraps) | GAP |
| - [ ] Bookmark gutter markers (blue circle) | GAP |
| - [ ] Clear all bookmarks command | GAP |

**Deliverables**: `src/bookmarks.rs`, gutter markers

**Unit tests**: toggle, navigate with wrap, clear all

---

### 2.5 Read-Only Mode (CE-22)

| Task | Status |
|------|--------|
| - [ ] Per-document read-only toggle (Edit menu) | GAP |
| - [ ] Tab indicator + status bar indicator | GAP |
| - [ ] Block all edit operations when active | GAP |

**Unit tests**: toggle, verify edits rejected

---

### 2.6 Auto-Save & Backup (FM-04, FM-05)

| Task | Status |
|------|--------|
| - [ ] Auto-save interval: off, 30s, 1m, 5m (configurable) | GAP |
| - [ ] Backup on save: copy previous version to `.bak` | GAP |

**Deliverables**: `src/autosave.rs`

**Unit tests**: timer logic, backup file creation/naming
**Integration test**: enable auto-save → modify → wait interval → verify saved

---

### 2.7 Named Sessions (FM-07)

| Task | Status |
|------|--------|
| - [ ] Save session with custom name | GAP |
| - [ ] Session manager dialog: list, switch, delete | GAP |
| - [ ] Store in `~/.codeedit/sessions/` | GAP |

**Deliverables**: `src/session_manager.rs`, session dialog

**Unit tests**: CRUD operations on session files

---

### 2.8 File Explorer Sidebar (FM-08)

| Task | Status |
|------|--------|
| - [ ] Tree-view panel (left side) showing folder contents | GAP |
| - [ ] Open folder as workspace root | GAP |
| - [ ] Click file → open in new tab | GAP |
| - [ ] Collapse/expand directories | GAP |
| - [ ] Basic file icons by type | GAP |

**Deliverables**: `src/file_explorer.rs`, sidebar panel

**Unit tests**: tree building from directory, filtering hidden files
**Headless UI test**: open folder → click file → verify tab opens

---

### 2.9 User Defined Language (SH-05, SH-06)

| Task | Status |
|------|--------|
| - [ ] UDL editor dialog: keywords, operators, comment styles, delimiters | GAP |
| - [ ] UDL export/import as XML | GAP |
| - [ ] Store in `~/.codeedit/udl/` | GAP |

**Deliverables**: `src/udl.rs`, UDL dialog

**Unit tests**: UDL parse, keyword matching, XML roundtrip

---

### 2.10 Bundled Themes (SH-09, CU-07)

| Task | Status |
|------|--------|
| - [ ] Bundled themes: Default Light, Default Dark, Solarized, Monokai, Zenburn | GAP |
| - [ ] Theme JSON format: token class → { fg, bg, bold, italic } | GAP |
| - [ ] Dark/light mode sync with Windows system setting | GAP |
| - [ ] Live preview on theme switch | GAP |

**Deliverables**: `themes/*.json` files, `src/themes.rs` expanded

**Unit tests**: theme loading, token resolution, dark mode API detection
**Screenshot test**: render same file with each theme → compare against baseline

---

### 2.11 Search Enhancements (SR-07, SR-08, SR-10, SR-11, SR-13)

| Task | Status |
|------|--------|
| - [ ] Replace in Files: preview panel before applying | GAP |
| - [ ] Incremental search: highlight as user types (real-time) | GAP |
| - [ ] Mark all matches (highlight without replacing) | GAP |
| - [ ] Named capture groups in regex replace (`$1`, `${name}`) | GAP |
| - [ ] Search history: last 50 terms, persisted to disk | GAP |

**Deliverables**: Replace preview panel, incremental search, search history

**Unit tests**: capture group substitution, history LRU, replace preview
**Integration test**: Replace in Files across temp dir → verify all files updated

---

### 2.12 Navigation Panels (CF-04, CF-05, CF-06, CF-07, CF-08)

| Task | Status |
|------|--------|
| - [ ] Custom fold markers: `// {{{` / `// }}}` | GAP |
| - [ ] Function List panel: regex-based function/class parsing per language | GAP |
| - [ ] Function List click → navigate to definition | GAP |
| - [ ] Function List search filter | GAP |
| - [ ] Document Map (minimap): scaled-down viewport indicator | GAP |

**Deliverables**: `src/function_list.rs`, `src/document_map.rs`

**Unit tests**: function regex for C/Rust/Python/JS, custom fold marker detection
**Headless UI test**: open Rust file → verify Function List contains `fn main`

---

### 2.13 Split View & Clone (MDI-05, MDI-06, MDI-07, MDI-09)

| Task | Status |
|------|--------|
| - [ ] Split editor horizontally or vertically (wire existing `SplitLayout`) | GAP (data model exists) |
| - [ ] Clone document: same buffer in two panes, edits sync | GAP |
| - [ ] Move document to other pane | GAP |
| - [ ] Pinned tabs: pin icon, prevent accidental close | GAP |

**Deliverables**: Split view UI, clone sync, pinned tab logic

**Unit tests**: split state, clone sync edits, pin prevents close
**Headless UI test**: split → edit pane A → verify pane B updates

---

### 2.14 Macros — Full UI (MA-01 through MA-08)

| Task | Status |
|------|--------|
| - [ ] Record macro UI: toolbar button, status bar "Recording" indicator | GAP (MacroRecorder exists) |
| - [ ] Stop and playback | GAP |
| - [ ] Run macro N times / until end of file | GAP |
| - [ ] Save macro with name to `~/.codeedit/macros/` (JSON) | GAP |
| - [ ] Assign keyboard shortcut to saved macro | GAP |
| - [ ] Built-in script runner: shell command on current file | GAP |
| - [ ] Output panel showing stdout/stderr | GAP |

**Deliverables**: Macro UI, `src/macro_runner.rs`, output panel

**Unit tests**: record → replay, run N times, save/load JSON
**Integration test**: record macro → save → load → replay → verify output

---

### 2.15 Plugin Loader (PL-01, PL-02, PL-06)

| Task | Status |
|------|--------|
| - [ ] Discover plugins in `plugins/` directory | GAP (manifest parse exists) |
| - [ ] Launch plugin subprocess with stdio transport | GAP (protocol exists) |
| - [ ] Plugin API: read/write document, cursor, selections, register menu items | GAP |
| - [ ] Plugin lifecycle: init on load, shutdown on close | GAP (PluginHost exists) |

**Deliverables**: `src/plugin_loader.rs`, real stdio transport

**Unit tests**: manifest discovery, capability validation
**Integration test**: launch mock plugin exe → RPC call → verify response

---

### 2.16 Tools & Line Operations (TU-01, TU-02, TU-03, TU-11, CU-02, CU-10)

| Task | Status |
|------|--------|
| - [ ] Column editor: fill column with numbers or text | GAP |
| - [ ] Sort lines: ascending, descending, case-insensitive | GAP |
| - [ ] Remove duplicate lines | GAP |
| - [ ] Remove blank lines | GAP |
| - [ ] Case conversion: UPPER, lower, Title, camelCase, snake_case | GAP |
| - [ ] Diff two open documents side by side (wire existing diff engine) | GAP |
| - [ ] Toolbar customization: add/remove/reorder buttons | GAP |
| - [ ] Per-language settings profile | GAP |

**Deliverables**: `src/line_operations.rs`, `src/case_convert.rs`, diff UI, toolbar config

**Unit tests**: sort, dedup, case conversion for all modes, diff rendering
**Headless UI test**: select lines → sort ascending → verify order

---

### Phase 2 — Integration & E2E Tests

- [ ] **Plugin E2E**: install plugin → open file → plugin validates → result panel shows diagnostic
- [ ] **Macro E2E**: record → save → close app → reopen → load macro → replay → verify
- [ ] **Split view E2E**: split → navigate both panes → clone document → edit → verify sync
- [ ] **Theme E2E**: switch all 5 themes → screenshot comparison → no rendering artifacts
- [ ] **File explorer E2E**: open project folder → expand tree → click file → verify opens in tab
- [ ] **Accessibility E2E**: full keyboard navigation through preferences, function list, file explorer

---

## Phase 3 — Ecosystem (All P2 Requirements)

**Goal**: Plugin ecosystem and power-user features.
**Timeline**: Months 7–9

---

### 3.1 Plugin Manager & Marketplace (PL-03, PL-04, PL-05, PL-07)

- [ ] Plugin Manager UI: browse, install, update, remove
- [ ] Plugin registry: hosted manifest JSON (HTTP fetch)
- [ ] Bundled plugins: Command Runner, JSON Viewer, XML Tools
- [ ] Plugin API: dockable panel registration

**Deliverables**: `src/plugin_manager.rs`, registry client

---

### 3.2 Advanced Completion & Calltips (CE-16, CE-17)

- [ ] Auto-completion from API/symbol lists per language
- [ ] Calltips / parameter hints for functions

**Deliverables**: `src/symbol_completion.rs`

---

### 3.3 Persistent Undo (CE-14)

- [ ] Undo history survives save operations
- [ ] Optional: persist undo to disk

---

### 3.4 Multi-Select Occurrences (SR-12)

- [ ] Select word → Ctrl+D to add next occurrence to multi-selection
- [ ] Select All Occurrences command → multi-carets at all matches

---

### 3.5 VSCode Theme Import (SH-10)

- [ ] Parse VSCode `.json` theme format
- [ ] Map VSCode scopes to internal theme tokens

**Deliverables**: `src/theme_import.rs`

---

### 3.6 Embedded Language Highlighting (SH-13)

- [ ] JS/CSS inside HTML
- [ ] Template literal highlighting

---

### 3.7 Hex Viewer (TU-09)

- [ ] Hex editor mode: dual pane (hex | ASCII)
- [ ] Edit bytes in hex view
- [ ] Toggle between text and hex mode

**Deliverables**: `src/hex_viewer.rs`

---

### 3.8 Hash & Encode Utilities (TU-05, TU-06, TU-07, TU-08)

- [ ] Add `sha2`, `md-5`, `base64` crates
- [ ] MD5, SHA-1, SHA-256 of file or selection
- [ ] Base64 encode/decode
- [ ] URL encode/decode
- [ ] Insert timestamp at cursor

**Deliverables**: `src/utilities.rs`

---

### 3.9 Config Import/Export (CU-08)

- [ ] Export all config (settings, keybindings, themes, UDLs, macros) as ZIP
- [ ] Import ZIP to restore

**Deliverables**: `src/config_export.rs`

---

### 3.10 Scripting API (MA-09)

- [ ] Embedded JS engine (`boa_engine` crate)
- [ ] Scripting API: document text, cursor, commands
- [ ] Script console panel

**Deliverables**: `src/scripting.rs`

---

### 3.11 Cross-Platform Linux Build

- [ ] GitHub Actions CI for Linux
- [ ] Platform-specific fixes (file dialogs, paths)

---

### Phase 3 — Integration & E2E Tests

- [ ] Plugin marketplace: browse → install → use → update → remove
- [ ] Hex editor: open binary → edit hex → save → verify bytes
- [ ] Config export/import roundtrip
- [ ] Linux CI: same test suite passes on Ubuntu

---

## Phase 4 — Polish & Platform (P3 + NFR)

**Goal**: Production quality, accessibility, i18n, macOS.
**Timeline**: Months 10–12

---

### 4.1 Tab Extras (MDI-08, MDI-10, MDI-11)

- [ ] Tab groups / panel layout persistence
- [ ] Tab color coding (user-assignable)
- [ ] Thumbnail preview on hover

### 4.2 Breadcrumb / Scope Indicator (CF-09)

- [ ] Breadcrumb bar: File > Class > Function

### 4.3 Character Info Panel (TU-10)

- [ ] Unicode codepoint, UTF-8 bytes, HTML entity

### 4.4 Print with Syntax Highlighting (FM-10)

- [ ] Print preview + print preserving colors

### 4.5 macOS Build

- [ ] CI for macOS, Cmd key bindings, native menu bar

### 4.6 Accessibility Audit

- [ ] Full keyboard navigation, screen reader (AccessKit → MSAA / AT-SPI)
- [ ] High contrast mode

### 4.7 Internationalization

- [ ] Externalize all UI strings
- [ ] 5 languages: English, Chinese, Japanese, German, French
- [ ] Language selector in preferences

**Deliverables**: `src/i18n.rs`, `locales/*.json`

### 4.8 Performance Benchmarking

- [ ] `criterion` benchmark suite: file open, syntax render, search, memory
- [ ] NFR targets: 100 MB < 2s, highlight < 16ms, idle < 50 MB, startup < 500ms
- [ ] CI regression gate

### 4.9 Crash Recovery

- [ ] Auto-save to recovery directory on interval
- [ ] On next launch: detect recovery files → prompt restore

### 4.10 Portable Mode

- [ ] Detect `portable.ini` → store config in app directory, no registry

### 4.11 Plugin Auto-Update (PL-08)

- [ ] Check for plugin updates on launch
- [ ] One-click update

---

### Phase 4 — Integration & E2E Tests

- [ ] Accessibility: automated keyboard-only navigation audit
- [ ] Performance: all benchmarks pass NFR thresholds in CI
- [ ] Cross-platform: Windows + Linux + macOS smoke tests
- [ ] i18n: all 5 languages complete and verified
- [ ] Crash recovery: kill process → relaunch → verify no data loss
- [ ] Portable mode: run from USB drive → verify no external writes

---

## Appendix: Requirement Traceability Matrix

Maps every PRD requirement ID to an implementation plan section.

| PRD ID | Description | Plan Section | Phase |
|--------|-------------|-------------|-------|
| CE-01 | 4 GB file support | 1.1 | 1 |
| CE-02 | Multi-caret | 1.3 | 1 |
| CE-03 | Column selection | 1.3 | 1 |
| CE-04 | Auto-indent | 1.4 | 1 |
| CE-05 | Smart indent | 2.1 | 2 |
| CE-06 | Brace matching/close | 1.4 | 1 |
| CE-07 | Word wrap | 1.3 | 1 |
| CE-08 | Line numbers | 1.3 | 1 |
| CE-09 | Whitespace viz | 1.5 | 1 |
| CE-10 | EOL conversion | 1.2 | 1 |
| CE-11 | Encoding support | 1.2 | 1 |
| CE-12 | Convert encoding | 1.2 | 1 |
| CE-13 | Undo/redo | 1.1 | 1 |
| CE-14 | Persistent undo | 3.3 | 3 |
| CE-15 | Auto-complete (doc) | 2.2 | 2 |
| CE-16 | Auto-complete (API) | 3.2 | 3 |
| CE-17 | Calltips | 3.2 | 3 |
| CE-18 | Zoom | 1.3 | 1 |
| CE-19 | Drag-drop text | 2.3 | 2 |
| CE-20 | Tab-to-spaces | 1.5 | 1 |
| CE-21 | Bookmarks | 2.4 | 2 |
| CE-22 | Read-only mode | 2.5 | 2 |
| CE-23 | Overtype mode | 1.3 | 1 |
| FM-01 | File CRUD | 1.6 | 1 |
| FM-02 | Recent files | 1.6 | 1 |
| FM-03 | External change | 1.6 | 1 |
| FM-04 | Auto-save | 2.6 | 2 |
| FM-05 | Backup on save | 2.6 | 2 |
| FM-06 | Session restore | 1.6 | 1 |
| FM-07 | Named sessions | 2.7 | 2 |
| FM-08 | File explorer | 2.8 | 2 |
| FM-09 | CLI args | 1.6 | 1 |
| FM-10 | Print | 4.4 | 4 |
| FM-11 | Drag files from OS | 1.6 | 1 |
| FM-12 | Save encoding/EOL | 1.2 | 1 |
| SH-01 | 50+ languages | 1.7 | 1 |
| SH-02 | Priority languages | 1.7 | 1 |
| SH-03 | Auto-detect lang | 1.7 | 1 |
| SH-04 | Manual lang override | 1.7 | 1 |
| SH-05 | UDL editor | 2.9 | 2 |
| SH-06 | UDL import/export | 2.9 | 2 |
| SH-07 | Style configurator | 1.7 | 1 |
| SH-08 | Global theme | 1.7 | 1 |
| SH-09 | Bundled themes | 2.10 | 2 |
| SH-10 | VSCode theme import | 3.5 | 3 |
| SH-11 | Highlight occurrences | 1.7 | 1 |
| SH-12 | Current line highlight | 1.7 | 1 |
| SH-13 | Embedded lang | 3.6 | 3 |
| SR-01 | Find bar | 1.8 | 1 |
| SR-02 | Find & Replace | 1.8 | 1 |
| SR-03 | Regex search | 1.8 | 1 |
| SR-04 | Case/whole-word | 1.8 | 1 |
| SR-05 | Wrap-around | 1.8 | 1 |
| SR-06 | Find in Files | 1.8 | 1 |
| SR-07 | Replace in Files | 2.11 | 2 |
| SR-08 | Incremental search | 2.11 | 2 |
| SR-09 | Results panel | 1.8 | 1 |
| SR-10 | Mark all matches | 2.11 | 2 |
| SR-11 | Regex capture replace | 2.11 | 2 |
| SR-12 | Multi-select occurrences | 3.4 | 3 |
| SR-13 | Search history | 2.11 | 2 |
| SR-14 | Go to Line | 1.8 | 1 |
| CF-01 | Syntax folding | 1.9 | 1 |
| CF-02 | Indent folding | 1.9 | 1 |
| CF-03 | Fold all/unfold all | 1.9 | 1 |
| CF-04 | Custom fold markers | 2.12 | 2 |
| CF-05 | Function List | 2.12 | 2 |
| CF-06 | Function List nav | 2.12 | 2 |
| CF-07 | Function List filter | 2.12 | 2 |
| CF-08 | Document Map | 2.12 | 2 |
| CF-09 | Breadcrumb | 4.2 | 4 |
| MDI-01 | Scrollable tabs | 1.10 | 1 |
| MDI-02 | Tab drag reorder | 1.10 | 1 |
| MDI-03 | Tab context menu | 1.10 | 1 |
| MDI-04 | Modified indicator | 1.10 | 1 |
| MDI-05 | Split view | 2.13 | 2 |
| MDI-06 | Clone document | 2.13 | 2 |
| MDI-07 | Move to other view | 2.13 | 2 |
| MDI-08 | Tab groups | 4.1 | 4 |
| MDI-09 | Pinned tabs | 2.13 | 2 |
| MDI-10 | Tab colors | 4.1 | 4 |
| MDI-11 | Tab thumbnails | 4.1 | 4 |
| MA-01 | Record macro | 2.14 | 2 |
| MA-02 | Playback macro | 2.14 | 2 |
| MA-03 | Run N times | 2.14 | 2 |
| MA-04 | Save/load macros | 2.14 | 2 |
| MA-05 | Macro shortcut | 2.14 | 2 |
| MA-06 | Macro XML storage | 2.14 | 2 |
| MA-07 | Script runner | 2.14 | 2 |
| MA-08 | Output panel | 2.14 | 2 |
| MA-09 | Scripting API | 3.10 | 3 |
| PL-01 | Plugin DLL loading | 2.15 | 2 |
| PL-02 | Plugin C API | 2.15 | 2 |
| PL-03 | Plugin Manager UI | 3.1 | 3 |
| PL-04 | Plugin marketplace | 3.1 | 3 |
| PL-05 | Bundled plugins | 3.1 | 3 |
| PL-06 | Plugin doc API | 2.15 | 2 |
| PL-07 | Plugin dockable panels | 3.1 | 3 |
| PL-08 | Plugin auto-update | 4.11 | 4 |
| CU-01 | Keyboard remap | 1.11 | 1 |
| CU-02 | Toolbar customize | 2.16 | 2 |
| CU-03 | Hide/show UI | 1.11 | 1 |
| CU-04 | Per-lang font | 1.11 | 1 |
| CU-05 | Global font | 1.11 | 1 |
| CU-06 | Preferences dialog | 1.11 | 1 |
| CU-07 | Dark/light sync | 2.10 | 2 |
| CU-08 | Config export/import | 3.9 | 3 |
| CU-09 | Tab size config | 1.5 | 1 |
| CU-10 | Per-lang settings | 2.16 | 2 |
| TU-01 | Column editor | 2.16 | 2 |
| TU-02 | Line operations | 2.16 | 2 |
| TU-03 | Case conversion | 2.16 | 2 |
| TU-04 | Trim whitespace | 1.12 | 1 |
| TU-05 | Insert timestamp | 3.8 | 3 |
| TU-06 | Hashing | 3.8 | 3 |
| TU-07 | Base64 | 3.8 | 3 |
| TU-08 | URL encode | 3.8 | 3 |
| TU-09 | Hex viewer | 3.7 | 3 |
| TU-10 | Char info panel | 4.3 | 4 |
| TU-11 | Diff viewer | 2.16 | 2 |
| TU-12 | Word/line count | 1.12 | 1 |
| TU-13 | Status bar info | 1.12 | 1 |
