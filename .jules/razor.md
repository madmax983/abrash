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
## [Reduction]
**Bloat:** Abstract error handling and missing const fns in geometry.rs, and unreadable literrals in fuzz_target.rs, and uninlined format args in havoc_arboretum.rs. Also imprecise_flops that had a good reason not to be changed (based on comments above the length method)
**Cut:** Added const to closest_point, grouped variable declarations and allowed imprecise_flops in math.rs, fixed literals and formatting.
**Saved:** Multiple compiler warnings, enforcing cleanly building codebase.
## [Reduction]
**Bloat:** Redundant clones and unreadable literals
**Cut:** Removed redundant clones of data variables in test, marked `Vec3::is_finite` as a const function as it calculates constants on copy-types, spaced large numbers out using '_' delimiters.
**Saved:** Multiple compiler warnings, enforcing cleanly building codebase.
