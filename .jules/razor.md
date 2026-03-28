## De-Abstract Renderer Trait
**Bloat:** The `Renderer` trait in `render_api/renderer.rs` was implemented by exactly one struct (`CpuRenderer`), adding an unnecessary layer of indirection and files.
**Cut:** Deleted `renderer.rs`, moved `RenderError` to `cpu_renderer.rs`, and merged the trait methods directly into `CpuRenderer`. Updated all examples to use the concrete struct.
**Saved:** 1 Trait, ~100 lines of boilerplate interface code, and reduced cognitive load by flattening the API surface.
## HasParent Trait Removal
**Bloat:** The `HasParent` trait was a single-implementation trait only used by `ProvisionalJointData` to abstract accessing the parent index during topological sorting.
**Cut:** Deleted the `HasParent` trait and its implementation. Updated `topological_sort_joints` to take `&[ProvisionalJointData]` directly.
**Saved:** 10 lines of code and the cognitive load of navigating an unnecessary trait boundary.
