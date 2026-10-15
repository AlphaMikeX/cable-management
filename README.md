# Cable Management

Native offline Rust application for Windows 10/11 x64. Map physical ports and their attached equipment without an account or internet connection.

Run `CableManagement.exe`. No adjacent runtime files or Rust installation are needed. The graphics driver must support OpenGL.

## Canvas workflow

1. Create a port-bearing device with **New device** in the left sidebar. Create separate devices only when their ports need mapping, for example a computer, hub or mixer.
2. Select **Edit** and a named side: **Front**, **Back**, **Left**, **Right**, **Top** or **Bottom**. Drag the device's edges or corner handles to resize the dimensions visible from that side. The hidden dimension and connector sizes stay unchanged.
3. Use **Rotate 90° clockwise** above the port palette to pre-rotate every library icon. Click again for another quarter turn; four clicks restore the original orientation. Drag any of the 20 connector icons onto the visible device surface. The preview and placed port use the selected angle; existing ports are unaffected. Drag a placed port to reposition it on that surface.
4. Double-click a port to name terminal equipment, such as **gaming mouse**. No extra device is created. Labels show only the attached equipment or connected device name; empty ports show **not connected**.
5. Drag labels anywhere on the canvas. A leader line follows the physical port, and the layout is saved with the project.
6. Right-click a port or label for **Rotate…**, **Connection…**, **Disconnect** or **Delete**. Rotation sets an absolute angle in degrees. The single **Connection…** dialog offers **Equipment name** for a mouse or other terminal equipment, or **Mapped device** for an existing device and one of its available physical ports. It loads the current connection for editing. Saving replaces that port's connection atomically; cancelling preserves it. Dragging a device from the left list onto a port preselects that device in the connection dialog.
7. Double-click a port connected to a mapped device to open that device immediately. Hover a linked port or its label to highlight the peer in the left list. Navigation works from either end of a cable.
8. Save as `.cable.json`. Reopening restores models, ports, connections, rotations and label positions. **Ctrl+Z** restores a deleted port and its connection or reverses an entire drag gesture.

Drag an empty area of the device left, right, up or down to turn it to the adjacent fixed side. A swipe of at least 35 screen pixels triggers one turn per mouse press; long or diagonal drags cannot leave the device at a free angle. Release and drag again for another turn. This works in View and Edit; ports, labels and resize handles retain their editing gestures. Dragging the surrounding canvas does not turn the device.

The right panel contains only the port palette. There is no properties inspector, bottom port list or tree view. Right-click a device in the left list to rename or delete it. Right-click the canvas title to add a box or wedge building block. Each building block has resize handles in Edit mode.

## Controls

| Control | Action |
|---|---|
| Drag a palette icon | Place a physical port; automatically enables Edit |
| Drag a port in Edit | Move it on its supporting surface |
| Drag an edge or corner in Edit | Resize the device in the current side view |
| Drag a label | Move its annotation, keeping the leader attached |
| Double-click a port or label | Name equipment, or navigate to a connected device |
| Rotate 90° clockwise above the palette | Pre-rotate library icons and future port drops |
| Right-click a port or label | Connection…, rotate, disconnect, delete |
| Drag a device from the left list onto a port | Connect to that device |
| Middle-button drag | Pan |
| Wheel | Zoom |
| Drag an empty device surface / named side buttons | Animated turns to fixed sides; no free rotation |
| Reset view | Restore the device's saved default side and fit |
| Ctrl+S / Ctrl+Shift+S | Save / Save as |
| Ctrl+O / Ctrl+N | Open / New |
| Ctrl+Z / Ctrl+Y | Undo / Redo |

Normal cables form a directed hierarchy: choose the computer port as source and the hub's upstream port as target. Select **Supplementary connection outside the hierarchy** for intentional additional links or cycles. Select **Adapter / converting cable** when connector families differ. This documents physical cable routing; it does not verify electrical or protocol compatibility.

## Storage and recovery

The [version-1 JSON format](docs/FORMAT.md) remains compatible with previous releases. Existing user names are preserved. The interface and newly generated content use English.

Saving validates the project, writes a temporary sibling and preserves the previous file as `.bak`. Windows uses atomic `MoveFileExW` replacement. A failed load leaves the current document intact. A failed save preserves the previous file.

Valid unsaved changes receive a recovery draft every 15 seconds at `%LOCALAPPDATA%\CableManagement\recovery.cable.json`. **Recover previous draft** appears on the next launch. Explicit saving or discarding changes clears recovery. A recovery draft never replaces an opened project automatically.

## Development

- `model.rs`: identities, physical port fit, atomic rotation/resizing and graph integrity.
- `geometry.rs`, `resize.rs`: surfaces, orthographic projection, visibility and fixed-side resize transforms.
- `scene.rs`: native canvas rendering, picking, port/label/device gestures and leader lines.
- `palette.rs`, `port_label.rs`, `ui.rs`: port library, contextual dialogs, device navigation, history and saving.
- `storage.rs`: validated versioned JSON, safe replacement and recovery.
- `tree.rs`: retained graph-layout utility; no tree view is exposed in the application.

The renderer draws projected polygons through egui's OpenGL backend. There is no Electron, WebView, HTML or JavaScript. Box/wedge modeling is schematic; this is not a full CAD editor or automatic hardware inventory tool.

On Windows, install Rust stable and Visual Studio Build Tools with C++ components:

```powershell
cargo fmt -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --release --locked
```

The executable is `target\release\CableManagement.exe`. GitHub Actions runs the Windows checks, builds the release and executes the native persistence diagnostic.

Linux cross-compilation requires MinGW-w64:

```sh
rustup target add x86_64-pc-windows-gnu
cargo build --release --locked --target x86_64-pc-windows-gnu
```

`.cargo/config.toml` links GNU runtime libraries statically. PE imports should contain only Windows system DLLs. Linux GUI checks need X11 and an X server. Windows uses native file pickers; Linux uses an in-app path dialog.

`CableManagement.exe --self-test <reportfile>` verifies the graph, labels, rotation, resizing, layout persistence, save/open and Windows file replacement without a graphics window. It writes temporary files beside the report and removes them afterward.

Source, tests and documentation belong in this repository. The Google Drive delivery folder contains only `CableManagement.exe`; user project and recovery files are created locally when the application is used.
