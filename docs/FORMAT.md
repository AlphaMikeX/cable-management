# Cable Management project format, version 1

UTF-8 JSON with a suggested `.cable.json` suffix. All physical dimensions are millimetres. Numeric identity values are positive, globally unique `u64`s. `next_id` must exceed every existing identity. Loading rejects unknown versions rather than guessing a migration.

| Top-level field | Meaning |
|---|---|
| `version` | `1` |
| `name` | Nonempty project title |
| `next_id` | Next unused global identity |
| `roots` | Every node without a normal incoming node connection |
| `nodes` | Port-bearing physical equipment |
| `endpoints` | Named terminal equipment, displayed as labels |
| `connections` | Directed cables reserving physical ports |

A node contains `id`, `name`, `parts`, `ports`, `default_view`, and `notes`. Its parts have `id`, `name`, `shape` (`Box` or `Wedge`), `size: [x,y,z]` and `offset: [x,y,z]`. Wedges slope down toward the front: the top is at `y/2` at the back and `0` at the front. Parts can overlap to form a schematic assembly.

A port contains `id`, `name`, `kind`, `part` identity, `face`, `uv`, `angle`, and `metadata`. Faces are `Front`, `Back`, `Left`, `Right`, `Top`, `Bottom`. `uv` is normalized surface-local position relative to the centre, with positive U to the right and positive V up when viewed from the outside of that face. Bounds account for the fixed connector dimensions and rotation. On wedge sides, normalized V is relative to the local height at U; on the wedge top it follows the slope. `angle` is degrees in the local surface plane. Contact size and outline come from the library, never from chassis scale.

`label_offset`, added in app 0.3, is optional on each port. Omission (or null) means automatic initial placement. Otherwise it is `[right, down]` in screen-plane millimetres relative to the projected socket centre. Zoom scales the offset; panning moves the socket and annotation together. Each component must be finite and within ±10,000 mm. Leaders always anchor to the physical socket. Old files load without conversion. Port label text is derived from the connected endpoint/device name, or exactly `not connected`, and is not a duplicate saved string.

A connection contains `id`, `from: {node,port}`, `to`, `supplementary`, `adapter`, and `label`. `to` is either `{"Port":{"node":2,"port":3}}` or `{"Endpoint":4}`. An endpoint contains `id`, `name` and `notes`. Ports at both ends and endpoint identities may occur in only one cable. Disconnecting terminal equipment deletes its now-unattached endpoint record.

Normal node cables form a directed forest: no cycles and at most one incoming hierarchy link per node. `supplementary: true` allows additional node cables, including graph cycles, while excluding that edge from the hierarchy projection. Both kinds share the same physical port occupancy constraints. Different contact families require `adapter: true`. USB socket shapes within the USB family are allowed to differ because the physical cable may have different connectors at either end. This does not infer protocol compatibility, voltage or direction.

The loader validates all identities, references, graph constraints, finite coordinates, port fit and roots before replacing the open document. Maximum file size is 32 MiB; maximum equipment count 5,000 and cable count 100,000. Future migrations must be explicit; the loader does not silently ignore a newer version.

Saving writes a temporary sibling, flushes its bytes, copies the old document to `<path>.bak` and replaces the original atomically. Failure before replacement leaves the original. A separate recovery document uses exactly the same schema and validation, without replacing the user's project.
