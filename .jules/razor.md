## De-Abstract Renderer Trait
**Bloat:** The `Renderer` trait in `render_api/renderer.rs` was implemented by exactly one struct (`CpuRenderer`), adding an unnecessary layer of indirection and files.
**Cut:** Deleted `renderer.rs`, moved `RenderError` to `cpu_renderer.rs`, and merged the trait methods directly into `CpuRenderer`. Updated all examples to use the concrete struct.
**Saved:** 1 Trait, ~100 lines of boilerplate interface code, and reduced cognitive load by flattening the API surface.
