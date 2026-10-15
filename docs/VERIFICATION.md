# v0.3.1 release verification — 2026-10-15

Code commit: `225d5a5` on `codex/cable-management-v0-1`.

## Checks

- Formatting and Clippy with warnings denied: passed.
- `cargo test --locked`: **67 tests passed**, including 32 native UI/rendering/gesture tests.
- Windows x64 GNU release cross-compilation: passed.
- Native persistence/canvas diagnostic: passed.
- [Windows CI](https://github.com/AlphaMikeX/cable-management/actions/runs/38793603329): passed: formatting, lint, all 67 tests, MSVC release build, native Windows persistence/canvas diagnostic and artifact upload.

The new pointer regressions exercise all four swipe directions from all six fixed views, in both View and Edit. They verify one turn per mouse press, dominant-direction diagonal swipes, long held drags and reversals, exact final yaw/pitch, another turn after releasing and pressing again, small-movement rejection and no rotation for drags starting off the device. A fast swipe whose movement and release share a frame also turns correctly.

The existing pointer suite still verifies that ports, labels and resize handles retain their gestures and that background primary/secondary dragging cannot change orientation. Swipes emit no project edit/history event. Every destination uses the same named-face camera target as the side buttons; pointer motion never assigns a free camera angle.

Independent review found that a fast release could be lost when swipe state began only after egui's drag threshold. A failing-then-passing regression now captures the eligible primary press before that threshold. The reviewer confirmed the fast-release fix and label/socket/resize ownership, and approved the final change. Egui still does not recognize a complete press/move/release batched into a single frame as a drag; this was reviewed as nonblocking.

## Native interaction check

Ran the native OpenGL application under X11. A leftward enclosure swipe changed Back to Left. A downward swipe changed Left to Top; holding the mouse down through the animation and extending the stroke retained Top, with no extra steps or free angle. The named side indicator matched the completed view. Labels and physical geometry remained intact. Interactive desktop checks used Linux; Windows CI separately executes the same headless gesture tests and native Windows persistence diagnostic.

## Delivery

`CableManagement.exe`, Windows x86_64 GNU release, **4,876,288 bytes**. Imports contain only Windows system DLLs; no adjacent runtime files are required.

SHA-256:

```text
f100966c003a6494d8856b5072e944003702e8855ab9e61bf4ef20eefde303d7
```

Replaced the existing executable in `AI Output/Cable Management` on Google Drive. A fresh download compared byte for byte equal to the build. The folder contains only `CableManagement.exe`. Source, tests and documentation are stored in GitHub.

The delivered GNU binary and the MSVC binary built in Windows CI use the same source but are different binaries. Project format and stored user data are unchanged in this patch release.
