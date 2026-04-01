## De-Abstract Renderer Trait
**Bloat:** The `Renderer` trait in `render_api/renderer.rs` was implemented by exactly one struct (`CpuRenderer`), adding an unnecessary layer of indirection and files.
**Cut:** Deleted `renderer.rs`, moved `RenderError` to `cpu_renderer.rs`, and merged the trait methods directly into `CpuRenderer`. Updated all examples to use the concrete struct.
**Saved:** 1 Trait, ~100 lines of boilerplate interface code, and reduced cognitive load by flattening the API surface.

## De-Abstract glTF Loader Provisional Trait
**Bloat:** The `HasParent` trait in `abrash-skeletal/src/gltf_loader.rs` was implemented by exactly one internal struct (`ProvisionalJointData`), adding an unnecessary abstraction layer for the topological sort function.
**Cut:** Deleted the `HasParent` trait and its implementation. Updated `topological_sort_joints` to take the concrete `ProvisionalJointData` slice directly and read its fields.
**Saved:** 1 Trait, ~10 lines of boilerplate interface code, reducing indirection in internal loading logic.
## 2024-05-18 - [Reduction]
**Bloat:** Complex, unreadable large numerical constants
**Cut:** Separated large literals with `_` (e.g. `0.000_000_1`)
**Saved:** Readability / Cognitive load

## De-Abstract GridMap Trait
**Bloat:** The `GridMap` trait in `abrash-raycast/src/map.rs` was implemented by exactly one struct (`ArrayGridMap`), adding an unnecessary abstraction layer.
**Cut:** Removed the `GridMap` trait and implemented its methods directly on `ArrayGridMap`. Replaced all dynamic usages with the concrete struct.
**Saved:** 1 Trait, ~15 lines of boilerplate interface code, reducing indirection in raycasting maps.
