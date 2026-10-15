# v0.2.0 release verification — 2026-10-15

Code commit: `031cfac` on `codex/cable-management-v0-1`.

## Automated checks

- `cargo fmt -- --check`: passed.
- `cargo clippy --all-targets --locked -- -D warnings`: passed.
- `cargo test --locked`: **41 tests passed** (16 native application/rendering, 7 geometry/layout, 7 model, 7 port interactions, 4 storage).
- `cargo build --release --locked --target x86_64-pc-windows-gnu`: passed.
- Native executable diagnostic on Linux: passed, including port placement, text labels, graph integrity and save/open replacement with backup.
- [Windows CI for this code](https://github.com/AlphaMikeX/cable-management/actions/runs/38659738547): passed: formatting, lint, all 41 tests, MSVC release compilation, native Windows persistence/label diagnostic and artifact upload.

The interactive tests exercise a real egui palette drag onto the device, double-click text editing with keyboard entry, the right-click Delete menu, removal and undo, and primary/secondary mouse drags preserving the fixed camera orientation. All six named side projections are covered. Text labels roundtrip without creating equipment nodes; cables to mapped hubs cannot be replaced accidentally.

Independent review found two issues: a selected neighbouring socket could win over a precise click, and a clamped drop could move behind another building block. Both received failing-then-passing regressions. The reviewer's actual context-menu reproduction now deletes the clicked socket, not its selected neighbour. An additional regression preserves unfinished equipment text while a close confirmation is open.

## Interactive checks

The native GUI ran under X11/OpenGL. A port icon was dragged from the right palette onto the existing PC, double-click opened the equipment dialog, and **gamingmus** was saved as an endpoint label with no extra node. Right-click Delete removed a port and Undo restored it. Existing version-1 projects opened without conversion. Palette icons, named sides, scrolling, text dialog and menu were visually inspected. Interactive desktop checks used Linux; the CI runner separately executes native Windows tests and the persistence diagnostic.

## Delivered executable

`CableManagement.exe`, Windows x86_64 GNU release, **4,896,768 bytes**. PE imports contain only Windows system DLLs, with no adjacent MinGW or other runtime files required.

SHA-256:

```text
55b166593d4e02005e0e812ef7afd932a258b8bc774a1b64d967ed0e141abbec
```

The file replaced the previous executable in the same Google Drive file and folder. A fresh download compared byte for byte equal to this build. The delivery folder contains only the executable; source, tests and documentation remain in GitHub.

The delivered GNU executable and the MSVC executable built by Windows CI come from the same Rust code, but are different binaries. This release retains the documented schematic box/wedge modeling limits. Model rotation is now limited to named side selection with animated transitions; free mouse orbit is removed.
