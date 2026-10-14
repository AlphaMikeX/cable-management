# Cable Management design

The attached development prompt is the product brief. The user requests a compiled Windows executable in Google Drive/AI Output/Cable Management, with source code stored in AlphaMikeX/cable-management on GitHub. Only runtime files belong in that Drive folder.

## Architecture and choices

Native Rust, egui/eframe and its OpenGL backend. A small Rust schematic 3D renderer projects and depth-sorts opaque chassis polygons and physical port outlines into egui's GPU draw list. This avoids a second graphics engine and keeps packaging to one executable. It supports orthographic orbit, zoom, animated fixed views, picking and face-local dragging. It is schematic visualization, without CAD boolean operations or photorealism.

Modules: model (identity, graph integrity), ports (extensible typed connector catalogue, millimetres), geometry (face mapping and projection), storage (versioned validated JSON and recovery), tree (full graph projection), ui (editing and navigation), scene (drawing and interaction).

## Data

Project version 1 contains globally unique integer IDs, nodes, endpoints, directed port connections and root IDs. A node contains editable box/wedge building blocks and ports attached by part ID, face, normalized local coordinates and angle. Physical connector dimensions are independent of the chassis. Endpoints only supply labels. Connections reserve both physical ports; ordinary hierarchy links are acyclic and have one parent per node. Explicit supplementary links permit real-world non-tree cabling and are shown separately. USB and audio connector families permit different plug shapes; other families require an explicit adapter flag.

## User flows

Swedish desktop UI, a permanent direct node list, top-level whole-tree/level toggle and explicit edit/view toggle. The inspector edits node names, chassis parts, ports, fixed views and cable targets. New projects start empty with clear creation controls; the welcome demo provides PC → hub → webcam, plus HDMI 1/2. Port selection shows both identities, endpoint and root route. All graph views derive directly from Project. Whole-tree navigation has no collapse state; supplementary edges do not duplicate hierarchy nodes.

Mouse orbit, wheel zoom, six animated fixed orientations, default orientation per node, port picking, face-local dragging in edit mode and numeric precision controls. Parts can be resized and offset; ports retain size and stay within their supporting face. Invalid sizes, duplicate IDs, dangling references, occupied ports, incompatible cables, cycles and unsupported versions yield errors without partial mutation.

## Persistence and delivery

Explicit save/open, native file dialogs on Windows and a native path dialog on Linux. Writes validate the document, write a sibling temporary file, preserve the previous version as .bak and replace safely. Recovery is saved separately under the user's local application data; it never silently replaces the main project. Opening/new/closing protects unsaved work. Undo/redo tracks document edits.

Windows x86_64 GNU release, static C runtime and only Windows system DLL imports. Embedded fonts and demo data; no installer, cloud, webview, external data files or network access at runtime. GitHub retains source, lockfile, tests, build workflow and docs. Drive receives only CableManagement.exe; the development prompt already in that delivery folder moves into AI Output so it is preserved outside the runtime folder.

## Verification

Tests exercise port identity and scale, graph constraints and supplementary links, endpoint routes, graph projection, geometry transforms, resize stability, JSON roundtrip, invalid loads and safe replacement. Compile and lint Rust; build the Windows PE; inspect imports; run a GUI smoke test where Wine is available. Report whether testing used Wine or real Windows. First release limitations are schematic meshes, manual cable entry and no hardware scanning or photograph import, as scoped by the brief.
