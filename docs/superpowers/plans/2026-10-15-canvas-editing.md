# Canvas Editing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deliver version 0.3 with canvas-only editing and English UI as requested.

**Architecture:** Keep the native egui renderer and version-1 JSON model. Add optional per-port label offsets, atomic rotation/resizing operations, scene gesture events and focused contextual dialogs. Independent model and UI files can be implemented concurrently with the renderer.

**Tech Stack:** Rust, eframe/egui, serde JSON, static Windows executable.

**Spec:** docs/superpowers/specs/2026-10-15-canvas-editing.md

## Global Constraints

- Existing projects must load without migration; new layout field has a serde default.
- Port labels show connected names only, otherwise `not connected`.
- All interface text in English; user content retains its original text.
- Camera direction changes only through the six named side buttons.
- Source lives in GitHub; delivery folder contains only executable runtime files.

## Review Focus

- Resize gestures freeze projection scale and camera center to avoid feedback as bounds change.
- Invalid rotations/resizes leave the model unchanged, never remove cables or ports.
- Overlapping small socket hit targets select the actual hovered physical port.
- Label dragging cannot move the port or orbit the camera; persisted offsets survive reload.
- Double-click and hover resolve connections in either direction; unsaved dialogs preserve text.

### Task 1: Persistent editing model

Files: src/model.rs, src/storage.rs, src/ports.rs, tests/canvas_model.rs.

- [x] Test old JSON without label_offset, offset roundtrip, connection-only labels and bidirectional peers.
- [x] Implement Port.label_offset: Option<[f32;2]>, Project::connected_peer and Project::port_label.
- [x] Test atomic failure and retained port/cable identities for rotation and resize.
- [x] Implement Project::rotate_port(r, degrees) and Project::resize_part(node, part, size, offset).
- [x] Translate generated names and errors; run core regression tests.

### Task 2: Canvas gestures

Files: src/scene.rs, src/palette.rs, new src/resize.rs.

- [x] Test resize edge/corner projection for all six views and invalid input.
- [x] Implement frozen drag transform and two-axis resize events, independent label drags with leader lines.
- [x] Extend port context actions for rotation, connection and disconnect; accept dragged nodes.
- [x] Test actual pointer gestures, precise picking, label content and manual orbit prevention.
- [x] Translate renderer/palette copy and use entire right panel for palette.

### Task 3: Canvas-only application shell

Files: src/ui.rs, src/port_label.rs, optional contextual dialog module.

- [x] Remove inspector/tree/bottom list; simplify obsolete editing buffers.
- [x] Implement contextual rotate/connect/rename dialogs and device-list highlights.
- [x] Apply gesture events with one undo snapshot per drag and validate candidates.
- [x] Test node navigation, connection/rotation dialogs, text preservation and save/undo.
- [x] Translate all application copy and update help.

### Task 4: Verification and delivery

Files: Cargo.toml, Cargo.lock, README.md, docs/CHANGELOG.md, docs/FORMAT.md, docs/VERIFICATION.md.

- [x] Run format, clippy, full tests and visual native interaction checks.
- [x] Obtain independent code review and fix important findings with regression tests.
- [ ] Cross-compile static Windows executable, push source and confirm Windows CI self-test.
- [ ] Replace existing Drive executable, download it and compare bytes; verify folder contents.
- [ ] Record final checks and artifact identity in repository.
