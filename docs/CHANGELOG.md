# Changes

## 0.2.0 — 2026-10-15

- A right-side palette provides icons for every connector type. Drag an icon onto the visible physical device surface to place a correctly scaled port. A preview highlights the supporting face. The palette scrolls through all 20 types and activates editing/level mode when a drag starts.
- Right-click a placed port or its label and choose **Delete · Ta bort port**. Port and cable are removed together; undo restores both. Picking distinguishes actual socket outlines from enlarged click tolerances, including closely spaced ports.
- Double-click a port or its label to type attached equipment, e.g. **gamingmus**. Saving creates or renames a plain endpoint label, without adding a port-bearing node. Clearing a label disconnects it. A cable to an already mapped node is preserved and offers navigation to that node.
- Free mouse rotation is removed. Model orientation changes only with the six named side controls or the saved standard view, retaining smooth animation. Zoom, panning and surface-local port dragging remain available.
- Dropping on empty space, a surface too small for the connector, or a clamped position hidden by another building block does not modify the document.
- Existing project format version 1 is unchanged. Node, port, cable and endpoint identities remain compatible with 0.1.0 projects.

Verification includes 40 automated tests: native palette-to-canvas drag, double-click text entry, context-menu Delete, undo, fixed mouse orientation, all six surface projections, fit/occlusion, graph integrity and local file roundtrips. Windows CI also builds the native executable and runs its expanded persistence/label self-test.
