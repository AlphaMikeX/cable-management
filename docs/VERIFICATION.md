# v0.3.0 release verification — 2026-10-15

Code commit: `de5d212` on `codex/cable-management-v0-1`.

## Automated checks

- `cargo fmt -- --check`: passed.
- `cargo clippy --all-targets --locked -- -D warnings`: passed.
- `cargo test --locked`: **63 tests passed**: 28 native application/rendering/resize tests, 10 canvas model tests, 7 geometry/layout tests, 7 model tests, 7 port interaction tests and 4 storage tests.
- `cargo build --release --locked --target x86_64-pc-windows-gnu`: passed.
- Native executable self-test on Linux: passed, including rotation, enclosure resize, persisted label offsets, graph/labels and save/open replacement with backup.
- [Windows CI for this code](https://github.com/AlphaMikeX/cable-management/actions/runs/38791442825): passed: formatting, lint, all 63 tests, MSVC release build, native Windows persistence/canvas diagnostic and artifact upload.

The pointer tests cover real palette placement, independent label dragging, a leader anchored to the physical socket, edge resizing across multiple frames without projection feedback, real typed rotation input with Enter, and named camera orientation. Model tests cover all six side axes, boxes/wedges, connector fit, atomic rejection, both directions of mapped links, old JSON compatibility and persisted annotation layout.

Independent review found two interaction bugs: a neighbouring label could intercept a physical socket's click/drag/double-click, and Undo during a held resize could be overwritten by the ongoing gesture. Both received failing-then-passing regression tests. Physical sockets now receive priority over annotations, default annotations avoid sockets and applying history ends active gestures. Independent pointer reproductions for both fixes and a node drop onto a label-covered socket passed. Review approved the resulting diff.

## Interactive checks

The native application ran under X11/OpenGL at 1600×1000 and the minimum 1080×720 window size. The right panel contains only the port palette; no properties, tree view or bottom port list remains. All 20 icons are accessible through the palette's scroll area.

Manually exercised independent label dragging with a leader, enclosure edge resizing in Edit mode, mapped-port hover highlighting the peer in the sidebar, double-click navigation to the mapped hub, the right-click Rotate dialog with typed 45° input, and dragging a device from the list onto a free port to preselect the connection target. Saving the resulting scratch project retained the resized dimensions, label offset and rotation. A fresh project and built-in example displayed English text; existing Swedish user names in an older project remained intact.

Interactive desktop checks used Linux. Windows CI separately builds and executes native Windows checks; it does not provide a manual graphical Windows session.

## Delivered executable

`CableManagement.exe`, Windows x86_64 GNU release, **4,873,216 bytes**. PE imports contain only Windows system DLLs; no adjacent MinGW or other runtime files are required.

SHA-256:

```text
f73a04027e058719b6373347030724786eae127cfaef9fb41b0557a5821d09f7
```

The new executable replaced the previous bytes in the same Google Drive file in `AI Output/Cable Management`. A fresh download compared byte for byte equal to this build. The folder contains only `CableManagement.exe`. Source, tests and documentation remain in GitHub.

The delivered GNU executable and the MSVC executable built by Windows CI come from the same Rust code but are different binaries. Existing version-1 projects open without conversion. The optional label offset is documented in FORMAT.md.
