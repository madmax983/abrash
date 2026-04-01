## De-Abstract Renderer Trait
**Bloat:** The `Renderer` trait in `render_api/renderer.rs` was implemented by exactly one struct (`CpuRenderer`), adding an unnecessary layer of indirection and files.
**Cut:** Deleted `renderer.rs`, moved `RenderError` to `cpu_renderer.rs`, and merged the trait methods directly into `CpuRenderer`. Updated all examples to use the concrete struct.
**Saved:** 1 Trait, ~100 lines of boilerplate interface code, and reduced cognitive load by flattening the API surface.
## [Reduction]
**Bloat:** The `HasParent` trait in `gltf_loader.rs` was an unnecessary abstraction used solely to provide a generic topological sort function for a single internal struct `ProvisionalJointData`.
**Cut:** Removed the `HasParent` trait entirely and implemented the required `parent_index` method directly on `ProvisionalJointData`. Updated `topological_sort_joints` to take a slice of `ProvisionalJointData` directly.
**Saved:** 1 Trait, 5 lines of boilerplate interface code, and reduced cognitive load by flattening the sorting logic.

## [Reduction]
**Bloat:** The `TimelineState` enum in `timeline.rs` wrapped a simple boolean completed state alongside the last evaluated sample, which was already being tracked in the parent struct.
**Cut:** Flattened `TimelineState` into a single `completed: bool` field directly on the `Timeline` struct and removed the enum.
**Saved:** 1 Enum, simplified match statements into straightforward `if self.completed` guard clauses.
