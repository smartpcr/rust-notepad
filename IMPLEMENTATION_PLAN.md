# Rust Notepad Implementation Plan (Derived from README)

This plan translates `README.md` roadmap into executable phases and tracks current implementation progress in this repository.

> **Quality gate target:** every phase should reach 100% line and branch coverage for the code introduced in that phase.

## Progress snapshot (synced)

- [x] **Phase 0 module implemented** (`src/phase0.rs`)
- [x] **Phase 1 module implemented** (`src/phase1.rs`)
- [x] **Phase 2 module implemented** (`src/phase2.rs`)
- [x] **Phase 3 module implemented** (`src/phase3.rs`)
- [x] **Core UI app scaffold present** (`src/app.rs`, `src/main.rs`)
- [ ] **Per-phase 100% coverage CI jobs enforced** (`phase-1-coverage`, `phase-2-coverage`, etc.)
- [ ] **Formal milestone sign-off artifacts recorded**

## Timeline diagram

```mermaid
gantt
    title Rust Notepad delivery timeline (plan vs repo progress)
    dateFormat  YYYY-MM-DD
    axisFormat  %m/%d

    section Foundation
    Phase 0: Foundation scaffold           :done, p0, 2026-01-06, 14d

    section MVP
    Phase 1: MVP editor workflow           :done, p1, after p0, 21d

    section Productivity
    Phase 2: Productivity enhancements     :done, p2, after p1, 28d

    section Power-user
    Phase 3: Extensibility features        :done, p3, after p2, 35d

    section Hardening
    Coverage CI + milestone sign-off       :active, harden, after p3, 14d
```

## Global engineering and quality policy

- [x] **Architecture target:** Rust desktop app with modular backend + editor-focused UI.
- [ ] **Definition of done (all phases):**
  - [x] Acceptance criteria implemented at module level.
  - [x] Unit/integration tests present for introduced logic.
  - [x] `cargo test` passes for touched modules.
  - [ ] Coverage gate passes at 100% line + branch on changed modules.
  - [ ] Regression tests added for every bug fixed during execution.
- [ ] **Coverage tooling fully wired:**
  - [ ] `cargo llvm-cov --all-features --workspace --branch --fail-under-lines 100 --fail-under-regions 100`
  - [ ] Frontend/editor integration coverage thresholds for changed files.
- [ ] **CI gating policy:**
  - [ ] No merge unless all checks pass and 100% phase coverage threshold is met.
  - [ ] Dedicated per-phase coverage jobs (`phase-1-coverage`, `phase-2-coverage`, etc.).

---

## Phase 0 — Foundation and architecture scaffold

### Goals
- [x] Establish project structure and contracts before feature build-out.
- [x] Create seams for file I/O, tabs, search, diagnostics, settings, plugin support.

### Deliverables
- [x] Core domain models (`Document`, `TabId`, `SessionState`, `SearchQuery`, `Diagnostic`).
- [x] Base error model and clock abstractions.
- [~] Full service-layer abstraction set (`FileService`, `SessionService`, etc.) as formal traits.

### 100% coverage plan
- [x] Unit tests for constructors and state transitions.
- [ ] Property tests for serialization/deserialization invariants.
- [~] Contract tests via in-memory fakes.
- [~] Explicit branch tests for all error paths.

### Exit criteria
- [x] Foundational modules created.
- [ ] Coverage report demonstrates 100% line + branch for Phase 0 modules.

---

## Phase 1 — MVP editor workflow

### Goals
- [x] Open / Save / Save As
- [x] Multi-tab with dirty markers
- [x] Close / Close All / Close Others
- [x] Syntax highlighting mapping
- [x] Current-tab find/replace
- [x] External-change detection + reload flow

### Deliverables
1. **Document lifecycle**
   - [x] Open existing file into a new tab.
   - [x] Save semantics for named and untitled docs.
   - [x] Dirty state updates on content mutation.
2. **Tab management**
   - [x] Create untitled tabs.
   - [x] Close current tab, close all, close others.
   - [~] Unsaved-change confirmation policy (basic behavior exists, richer UX pending).
3. **Editing UX baseline**
   - [x] Syntax detection from extension.
   - [~] Manual syntax override support.
4. **Find/replace (active tab)**
   - [x] Case-sensitive and whole-word options.
   - [x] Replace current / Replace all core logic.
5. **External file change handling**
   - [x] Detection support exists.
   - [x] Clean document reload path.
   - [~] Dirty conflict policy UX can be expanded.

### 100% coverage plan
- [x] Unit tests for dirty flags, tab behavior, syntax map, find/replace logic.
- [x] Integration-style file open/edit/save tests.
- [~] E2E tests for full user flows.
- [~] Negative-path tests including write failure and watcher edge cases.

### Exit criteria
- [x] MVP behavior aligned with README scope at code level.
- [ ] 100% line + branch coverage enforcement for all Phase 1 code.

---

## Phase 2 — Productivity enhancements

### Goals
- [x] Find in open tabs
- [x] Session restore serialization primitives
- [x] Recent files LRU
- [x] JSON/XML formatting + validation
- [x] Theme and keybinding customization primitives

### Deliverables
1. **Search in open tabs**
   - [x] Unified search hit model.
   - [~] Navigation wiring to UI selection.
2. **Session restore**
   - [x] Persist/restore primitives for tabs + selection.
   - [~] Crash-safe persistence policy hardening.
3. **Recent files**
   - [x] LRU with deduplication and prune support.
4. **JSON/XML tools**
   - [x] JSON format + validation.
   - [x] XML validation diagnostics.
5. **Customization**
   - [x] Theme/keybinding settings model.
   - [x] Keybinding conflict detection.

### 100% coverage plan
- [x] Unit tests for LRU, session roundtrip, formatter/validator, keybinding conflicts.
- [~] Integration tests for restart simulation and end-to-end diagnostics pipeline.
- [~] E2E tests for startup restore + relaunch persistence.
- [~] Corruption/failure fallback tests.

### Exit criteria
- [x] Phase 2 feature primitives present and test-covered.
- [ ] 100% line + branch coverage enforcement for Phase 2 modules.

---

## Phase 3 — Extensibility and power-user features

### Goals
- [x] Plugin SDK foundations (manifest parsing)
- [x] Project-wide search
- [x] Split pane UX
- [x] Diff view primitives
- [x] Command palette/macros core

### Deliverables
1. **Plugin SDK (process-based direction)**
   - [x] Plugin manifest schema + parsing.
   - [x] JSON-RPC protocol transport and lifecycle.
   - [x] Capability sandbox + timeout policy primitives.
2. **Project-wide search**
   - [x] Recursive file scanning.
   - [x] Include/exclude filters.
   - [x] Streaming results for very large projects.
3. **Split panes**
   - [x] Side-by-side and stacked layouts.
   - [x] Independent cursor/scroll state.
4. **Diff view**
   - [x] Line diff operation model.
   - [x] Inline/side-by-side UI rendering modes.
5. **Command palette/macros**
   - [x] Command registry + search.
   - [x] Macro record/replay ordering.

### 100% coverage plan
- [x] Unit tests for manifest parsing, diff ops, command routing, macro replay.
- [x] Integration tests for plugin lifecycle and large project search.
- [x] E2E flow for plugin invocation, split panes, project search, diff rendering, and macro replay.
- [ ] Security/fuzzing tests for malformed plugin protocol payloads.

### Exit criteria
- [x] Core Phase 3 primitives implemented.
- [ ] 100% line + branch coverage enforcement for all Phase 3 additions.

---

## Cross-phase non-functional track

- [~] **Performance budgets:** basic implementation exists; formal budgets pending.
- [~] **Reliability:** foundational save/session behavior exists; crash recovery hardening pending.
- [~] **Accessibility/UX consistency:** keyboard-centric actions exist; full audit pending.
- [ ] **Packaging:** reproducible and signed release pipeline pending.

### Non-functional coverage enforcement
- [ ] Each non-functional module has full unit/integration coverage and phase gate inclusion.

---

## Milestone cadence and current state

- [x] **Milestone A (Weeks 1–3):** Phase 0 + core Phase 1 groundwork.
- [x] **Milestone B (Weeks 4–6):** Phase 1 completion baseline.
- [x] **Milestone C (Weeks 7–10):** Phase 2 primitives + stabilization.
- [x] **Milestone D (Weeks 11–15):** Phase 3 primitives implemented.
- [ ] **Milestone E (Hardening):** CI coverage gates, release-quality E2E, sign-off artifacts.

Every milestone should close with:
1. [ ] Feature acceptance sign-off.
2. [ ] 100% coverage report for milestone modules.
3. [ ] Regression suite green in CI.
