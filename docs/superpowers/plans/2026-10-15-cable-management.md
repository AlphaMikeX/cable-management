# Cable Management Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task by task.

**Goal:** Deliver the native offline Rust cable mapping application and a self-contained Windows executable.

**Architecture:** Shared versioned Project model, separate validation, storage and geometry, native egui inspector and two navigation modes. A schematic 3D renderer emits meshes into eframe's OpenGL draw list.

**Tech Stack:** Rust 2024, serde/JSON, egui/eframe 0.33, OpenGL, Windows GNU toolchain.

**Spec:** docs/superpowers/specs/2026-10-15-cable-management-design.md

## Global Constraints
- Native Rust UI, no web technologies or runtime network requirement.
- Endpoints are labels; nodes own physical ports.
- All port dimensions in millimetres, independent of chassis scaling.
- Only runtime files in Google Drive/AI Output/Cable Management.
- Source, tests, docs and Cargo.lock in GitHub.

## Review Focus
- Invalid or newer documents must not replace the open project.
- Part deletion must remove dependent ports and cables without dangling endpoints.
- Multiple roots, detached nodes and supplementary links must remain reachable and visible.
- Save failure must retain both the previous file and the dirty in-memory document.
- Hidden ports must not be picked through opaque chassis surfaces.

### Task 1: Document model, graph and connector catalogue
Files: src/lib.rs, src/model.rs, src/ports.rs, tests/model.rs.
Interfaces: Project::connect(PortRef, Target, bool, bool) -> Result<Id, String>; Project::validate() -> Result<(), String>; Project::route_to_endpoint(Id) -> Vec<PortRef>.
- [ ] Write tests for physical port occupancy, unique IDs, compatible families, adapter opt-in, acyclic hierarchy, supplementary links, endpoint routes and deletion.
- [ ] Observe missing behavior, implement model and catalogue, run cargo test --no-default-features.
- [ ] Commit verified model.

### Task 2: Geometry, projection and storage
Files: src/geometry.rs, src/tree.rs, src/storage.rs, tests/geometry.rs, tests/storage.rs.
Interfaces: face_frame(part, face, uv) returns centre and orthonormal tangents; project(point, camera) returns position/depth; tree::layout(&Project) returns node positions; storage::save/load use Path and Result.
- [ ] Write geometry/resize/route/tree and versioned JSON/safe replacement tests and observe missing behavior.
- [ ] Implement the Rust transforms, cycle-safe layout and validated atomic storage; run full core tests.
- [ ] Commit verified core services.

### Task 3: Native application and physical editor
Files: src/main.rs, src/ui.rs, src/scene.rs.
Consumes: Project, catalogue, geometry, storage, tree interfaces above.
- [ ] Implement direct node list, full tree and level mode, edit/view controls, breadcrumb navigation, part/port/cable editor, port picking and drag, smooth views and zoom.
- [ ] Add undo/redo, save/open protection and separate recovery.
- [ ] Compile and run GUI smoke tests against the acceptance chain and file reopening.
- [ ] Commit native app and documented controls.

### Task 4: Windows release and delivery
Files: .cargo/config.toml, .github/workflows/windows.yml, README.md, docs/FORMAT.md.
- [ ] Run cargo fmt --check, cargo clippy --all-targets -- -D warnings and cargo test.
- [ ] Build locked release for x86_64-pc-windows-gnu and inspect PE imports.
- [ ] Run Windows executable under Wine if available and record verification honestly.
- [ ] Perform independent whole-project code review and fix important findings.
- [ ] Push source and build instructions to GitHub, upload only the executable to the confirmed Drive folder, and read back folder contents.
