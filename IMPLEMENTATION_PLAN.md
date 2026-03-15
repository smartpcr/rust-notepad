# Rust Notepad Implementation Plan (Derived from README)

This plan translates the product direction in `README.md` into executable phases, with a strict quality gate:

> **Every phase is complete only when the phase test suite reaches 100% line and branch coverage for the code introduced in that phase.**

## Global engineering and quality policy

- **Architecture target:** Rust desktop app with a modular backend and an editor-focused UI surface, aligned with the Notepad++-like scope.
- **Definition of done (all phases):**
  - All acceptance criteria implemented.
  - Unit/integration/E2E tests added for all new logic.
  - `cargo test` passes for all crates/modules touched.
  - Coverage gate passes at 100% line + branch coverage on changed modules.
  - Regression tests added for every bug fixed during phase execution.
- **Coverage tooling:**
  - Rust coverage via `cargo llvm-cov --all-features --workspace --branch --fail-under-lines 100 --fail-under-regions 100`.
  - Frontend/editor integration coverage (if added) via framework-native tooling (e.g., Vitest/Playwright) with threshold = 100% for files changed in phase.
- **CI gating policy:**
  - No merge unless all checks pass and 100% phase coverage threshold is met.
  - Enforce per-phase coverage in dedicated CI jobs (`phase-1-coverage`, `phase-2-coverage`, etc.).

---

## Phase 0 — Foundation and architecture scaffold

### Goals
- Establish project structure and contracts before feature build-out.
- Create stable seams for file I/O, tabs, search, diagnostics, settings, and plugin host.

### Deliverables
- Workspace/module layout:
  - `core` domain models (`Document`, `TabId`, `SessionState`, `SearchQuery`, `Diagnostic`).
  - `services` layer (`FileService`, `SessionService`, `SearchService`, `PluginService`).
  - `ui` adapter layer (commands/events/view models).
- Error model and telemetry hooks.
- Test harness utilities:
  - temp files/dirs fixtures,
  - fake clock abstraction,
  - deterministic filesystem watcher harness,
  - snapshot helpers for serialized session state.

### 100% test coverage plan
- Unit tests for every domain model constructor and state transition.
- Property tests for serialization/deserialization invariants.
- Contract tests for service traits using in-memory fakes.
- Branch tests for all error paths (permission denied, missing file, invalid encoding).

### Exit criteria
- All foundational modules created and documented.
- Coverage report shows 100% line+branch for Phase 0 modules.

---

## Phase 1 — MVP editor workflow (README Phase 1 scope)

### Goals
Implement:
- Open / Save / Save As
- Multi-tab with dirty markers
- Close / Close All / Close Others
- Syntax highlighting
- Current-tab find/replace
- External-change detection + auto-reload flow

### Deliverables
1. **Document lifecycle**
   - Open existing file into a new tab.
   - Save semantics for named and untitled docs.
   - Dirty state updates on any content mutation.
2. **Tab management**
   - Create untitled tabs.
   - Close current tab, close all, close others.
   - Unsaved-change confirmation policy.
3. **Editing UX baseline**
   - Syntax mode detection from extension.
   - Manual syntax override support.
4. **Find/replace (active tab)**
   - Case-sensitive and whole-word options.
   - Replace current / Replace all.
5. **External file change handling**
   - Watcher-driven detection.
   - If clean: auto reload.
   - If dirty: prompt conflict policy.

### 100% test coverage plan
- Unit tests:
  - dirty flag transitions,
  - tab close behavior edge cases,
  - syntax mapping fallback behavior,
  - find/replace match indexing and replacement correctness.
- Integration tests:
  - open-edit-save round trip with temp files,
  - external change simulation while clean vs dirty,
  - close-all/close-others with mixed dirty states.
- E2E tests:
  - user flow: open file → edit → save → reopen.
  - user flow: find/replace with multiple matches.
- Negative-path tests:
  - failed file write,
  - malformed path input,
  - watcher event storm deduplication.

### Exit criteria
- MVP behavior matches README Phase 1.
- Coverage remains 100% line+branch for all Phase 1 code.

---

## Phase 2 — Productivity enhancements (README Phase 2 scope)

### Goals
Implement:
- Find in open tabs
- Session restore
- Recent files
- JSON/XML formatting + validation
- Theme and keybinding customization

### Deliverables
1. **Search in open tabs**
   - Unified result model grouped by tab and line.
   - Click-through navigation to result location.
2. **Session restore**
   - Persist open tabs, cursor positions, selected tab, unsaved buffers policy.
   - Restore on app startup with crash-safe writes.
3. **Recent files**
   - LRU list with deduplication and missing-file pruning.
4. **JSON/XML tools**
   - Format/minify actions.
   - Validation diagnostics integrated into status area.
5. **Customization**
   - Theme persistence.
   - Configurable keybindings with conflict detection.

### 100% test coverage plan
- Unit tests:
  - LRU behavior,
  - session serialization and migration,
  - formatter idempotence where applicable,
  - keybinding conflict resolver.
- Integration tests:
  - session persist/restore across process restart simulation,
  - cross-tab search indexing and navigation,
  - diagnostics pipeline for valid/invalid JSON/XML samples.
- E2E tests:
  - startup restore flow,
  - theme and keybinding persistence after relaunch.
- Robustness tests:
  - corrupted session file fallback,
  - formatter failure handling on invalid input.

### Exit criteria
- README Phase 2 features complete and user-visible.
- 100% line+branch coverage on all Phase 2 modules.

---

## Phase 3 — Extensibility and power-user features (README Phase 3 scope)

### Goals
Implement:
- Plugin SDK (process-based preferred)
- Project-wide search
- Split panes
- Diff view
- Command palette/macros

### Deliverables
1. **Plugin SDK (process-based)**
   - Plugin manifest schema and discovery.
   - JSON-RPC protocol over stdio/socket.
   - Sandboxed capability model and timeout/cancellation.
2. **Project-wide search**
   - Recursive file scanning with include/exclude filters.
   - Regex + literal search modes.
   - Streaming results for large projects.
3. **Split panes**
   - Side-by-side and stacked layouts.
   - Independent cursor and scroll state.
4. **Diff view**
   - Inline and side-by-side diff modes.
   - File vs saved-state comparison.
5. **Command palette/macros**
   - Discoverable command registry.
   - Macro recording/replay with deterministic ordering.

### 100% test coverage plan
- Unit tests:
  - plugin manifest parsing and schema validation,
  - command routing,
  - diff algorithm branch coverage,
  - macro replay determinism.
- Integration tests:
  - plugin process lifecycle (spawn, handshake, timeout, crash recovery),
  - project search over synthetic directory trees,
  - split-pane state synchronization and isolation.
- E2E tests:
  - install/load plugin and execute command,
  - run project search and navigate results,
  - record and replay macro across tabs.
- Security/resilience tests:
  - plugin protocol fuzzing,
  - malformed RPC payload handling,
  - denial-of-service guardrail validation.

### Exit criteria
- README Phase 3 feature set available behind stable UX.
- 100% line+branch coverage across Phase 3 additions.

---

## Cross-phase non-functional track

Run this track in parallel with all phases to preserve quality:

- **Performance budgets**
  - Open 10 MB file under target latency.
  - Search response-time thresholds for tab and project scopes.
- **Reliability**
  - Crash-safe session writes.
  - Auto-recovery tests for interrupted save operations.
- **Accessibility and UX consistency**
  - Keyboard-first workflows for all key actions.
  - Status and diagnostics messaging standards.
- **Packaging**
  - Reproducible builds for Windows/macOS/Linux.
  - Signed artifacts and update channel strategy.

### Non-functional 100% coverage enforcement
- Each non-functional module (telemetry, recovery, settings migration, perf guards) must include full unit/integration coverage and be included in phase coverage gates.

---

## Suggested milestone cadence

- **Milestone A (Weeks 1–3):** Phase 0 + core of Phase 1.
- **Milestone B (Weeks 4–6):** Complete Phase 1 and harden tests.
- **Milestone C (Weeks 7–10):** Phase 2 features + stabilization.
- **Milestone D (Weeks 11–15):** Phase 3 + plugin hardening + release readiness.

Every milestone must close with:
1. Feature acceptance sign-off.
2. 100% coverage report for milestone modules.
3. Regression suite green in CI.
