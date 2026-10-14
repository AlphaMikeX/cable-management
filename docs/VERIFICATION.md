# v0.1.0 release verification — 2026-10-15

Code commit: `2fb728f` on `codex/cable-management-v0-1`.

## Automated checks

- `cargo fmt -- --check`: passed.
- `cargo clippy --all-targets --locked -- -D warnings`: passed.
- `cargo test --locked`: 25 tests passed (7 native application/rendering, 7 geometry/layout, 7 model integrity, 4 storage).
- `cargo build --release --locked --target x86_64-pc-windows-gnu`: passed.
- [Native Windows CI](https://github.com/AlphaMikeX/cable-management/actions/runs/38648706474): all steps passed, including formatting, linting, tests, MSVC release compilation and the executable's persistence self-test. The latter executes on the Windows runner and verifies project roundtrip, endpoint route and safe file replacement with backup.

An independent review identified oblique port draw ordering, preservation of pending form values, persistent endpoint/cable text buffers and sloping-side port fit. Each important finding received a regression test that failed before the correction and passed afterward. A further regression verifies that explicitly selected overlapping sockets retain their identity during drag.

## Interactive checks

The native GUI ran under X11 with software OpenGL. Checked welcome/example loading, direct node navigation, the PC → hub → webcam chain, animated front/back views, full-tree display, editing mode, creation of a new PC with two individually placed HDMI ports, native port-library selection, mouse placement, save and reopening. Windows persistence was additionally executed by the CI runner; interactive GUI checks were on Linux rather than a physical Windows desktop.

## Delivered executable

`CableManagement.exe`, Windows x86_64 GNU release, 4,855,808 bytes. PE imports contain only Windows system DLLs; no MinGW runtime DLLs or other adjacent files are required.

SHA-256:

```text
a4126d122abd8bd25b62553ddd610a885663ffc47ebacc28d200f13474dc5301
```

The uploaded file was downloaded from Google Drive and compared byte for byte with this artifact. The delivery folder contains only the executable. The development prompt originally in that folder was preserved by moving it to the parent AI Output folder.

The delivered GNU executable and the Windows-tested MSVC executable are builds of the same Rust code; they are different binaries. The CI workflow and complete source are in this repository. The GUI remains schematic box/wedge modeling as described in the product brief, without full CAD or automatic hardware inventory.
