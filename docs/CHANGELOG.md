# Changes

## 0.3.1 — 2026-10-15

- Drag an empty enclosure surface to turn to an adjacent fixed side. Horizontal and vertical swipes work from all six views, in View and Edit. Each mouse press triggers at most one animated turn after a 35-pixel threshold. The camera always finishes at a named side; there is no free rotation.
- Port movement, label dragging, resize handles and canvas panning keep their existing gestures. Background drags do not turn the model.


## 0.3.0 — 2026-10-15

- Canvas-only editing: removed properties inspector, bottom port list and tree view. Right panel contains only the full-height draggable port library. Entire interface is English.
- Drag enclosure edges and corners in Edit mode to resize the current view dimensions. A frozen projection keeps the opposite edge fixed throughout the gesture. Physical ports and cables retain their identities and dimensions.
- Right-click a port to set its rotation in degrees, connect it to another mapped device, disconnect it or delete it. Devices can also be dragged from the left list onto a port to start a connection.
- Labels show only the connected equipment/device name or `not connected`. Drag labels independently; leader lines remain anchored to the port and positions persist in the project.
- Double-click a mapped port to open the connected device immediately. Hover a linked port or label to highlight its peer in the left list. Both directions are supported.
- Existing version-1 files remain compatible through an optional per-port label offset. Source and documentation stay in GitHub; the delivery folder contains only the executable.

## 0.2.0 — 2026-10-15

- A right-side palette provides icons for every connector type. Drag an icon onto the visible physical device surface to place a correctly scaled port. A preview highlights the supporting face. The palette scrolls through all 20 types and activates editing/level mode when a drag starts.
- Right-click a placed port or its label and choose **Delete · Ta bort port**. Port and cable are removed together; undo restores both. Picking distinguishes actual socket outlines from enlarged click tolerances, including closely spaced ports.
- Double-click a port or its label to type attached equipment, e.g. **gamingmus**. Saving creates or renames a plain endpoint label, without adding a port-bearing node. Clearing a label disconnects it. A cable to an already mapped node is preserved and offers navigation to that node.
- Free mouse rotation is removed. Model orientation changes only with the six named side controls or the saved standard view, retaining smooth animation. Zoom, panning and surface-local port dragging remain available.
- Dropping on empty space, a surface too small for the connector, or a clamped position hidden by another building block does not modify the document.
- Existing project format version 1 is unchanged. Node, port, cable and endpoint identities remain compatible with 0.1.0 projects.

Verification includes 41 automated tests: native palette-to-canvas drag, double-click text entry, context-menu Delete, undo, fixed mouse orientation, all six surface projections, fit/occlusion, pending text during close confirmation, graph integrity and local file roundtrips. Windows CI also builds the native executable and runs its expanded persistence/label self-test.
