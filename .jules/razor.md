## De-Abstract Renderer Trait
**Bloat:** The `Renderer` trait in `render_api/renderer.rs` was implemented by exactly one struct (`CpuRenderer`), adding an unnecessary layer of indirection and files.
**Cut:** Deleted `renderer.rs`, moved `RenderError` to `cpu_renderer.rs`, and merged the trait methods directly into `CpuRenderer`. Updated all examples to use the concrete struct.
**Saved:** 1 Trait, ~100 lines of boilerplate interface code, and reduced cognitive load by flattening the API surface.
## De-Abstract GridMap, BspMap, BspTextures
**Bloat:** The `GridMap`, `BspMap`, and `BspTextures` traits added unnecessary layers of abstraction for data structures that currently only have one single meaningful implementation (or only existed for tests).
**Cut:** Deleted the traits and replaced them with concrete structs (`GridMap`, `BspMap`, `BspTextures`), moving their mock initialization logic to constructors (`new_mock`) for use in test environments.
**Saved:** 3 Traits, ~50 lines of boilerplate interface code, and simplified the rendering API surface by removing generic bounds (`impl GridMap`, `impl BspMap`, etc.).
