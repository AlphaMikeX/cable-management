# v0.4.0 release verification — 2026-10-16

Code commit: `3fa1461` on `codex/cable-management-v0-1`.

## Automated checks

- `cargo fmt -- --check`: passed.
- `cargo clippy --all-targets --locked -- -D warnings`: passed.
- `cargo test --locked`: **85 tests passed**, including 48 native UI/palette/rendering/editor tests and 37 integration tests.
- Windows x64 GNU release cross-compilation: passed.
- Native executable self-test: passed, including pre-rotated placement, persistence, rotation, resizing, labels and safe replacement.
- [Windows CI](https://github.com/AlphaMikeX/cable-management/actions/runs/38819331883): passed: formatting, lint, all 85 tests, MSVC release build, native Windows persistence diagnostic and artifact upload.

Palette tests exercise the actual rotation button above the icons, four-click cycling, captured drag angle and all 20 connector icon bounds. An asymmetric USB-B icon verifies clockwise direction. A UI integration test clicks the library button and drops a vertical USB-A onto a 6 mm wide face, where the unrotated port would not fit. Existing ports, dirty state and undo remain unchanged when only the library rotates. Preview and final placement use the same angle; invalid rotated fits preserve the complete project and identity counter. Angles persist through JSON reload.

Connection tests cover one common editor, equipment and mapped-device modes, atomic source replacement, retargeting and undo, same-peer receiving-side option edits preserving cable identity/direction/label, unchanged Save preserving all IDs, endpoint rename identity, current-peer inclusion despite port occupancy, occupied-target rejection, cancelling, incomplete draft Save-and-continue rejection, and preservation through error dismissal. Existing mapped-port double-click navigation works in either direction.

Independent review found a dropdown keyboard issue: Enter could save and Escape could discard the enclosing draft while a device/port popup was open. The regression failed first, then passed with popup-aware shortcuts and equipment-only Enter submission. The reviewer reproduced the fix independently and approved the final changes. No other actionable review findings remained.

## Native interaction checks

The application ran under X11/OpenGL at 1600×1000. The new button rotated library icons clockwise while the project stayed Saved. Dragging USB-A onto the chassis saved its angle as 270° in the existing surface-local counterclockwise convention; older ports retained their 0° angles. The context menu displayed exactly one **Connection…** entry. Its equipment-name mode saved `Desk mouse` without adding a node, and the same dialog switched to mapped-device and free-port selection.

Manual graphical checks used Linux. Windows CI separately runs the full headless egui gesture/editor tests and a native Windows persistence diagnostic. Project format is unchanged; existing user data remains compatible.

## Delivered executable

`CableManagement.exe`, Windows x86_64 GNU release, **4,886,528 bytes**. Imports contain only Windows system DLLs, so no adjacent runtime files are required.

SHA-256:

```text
0e4bb9192a46e64c0b860fd2726eca9a92ff69df3b2b3635614083f1dbbfeaa6
```

Replaced the same Google Drive file in `AI Output/Cable Management`. A fresh download compared byte for byte equal to the compiled build. The folder contains only `CableManagement.exe`; source, tests and documentation remain in GitHub.

The delivered GNU executable and the MSVC release built in Windows CI use the same source but are different binaries.
