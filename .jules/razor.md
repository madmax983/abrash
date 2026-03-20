## [Reduction]
**Bloat:** The `Renderer` and `AsciiExporter` traits. `Renderer` was a single-implementation trait used only by `CpuRenderer`. `AsciiExporter` was a single-implementation trait used only by `Framebuffer`.
**Cut:** De-abstracted `Renderer` by moving its logic directly into `CpuRenderer` and making it a concrete struct. De-abstracted `AsciiExporter` by converting its methods into standalone free functions that operate on `&Framebuffer`.
**Saved:** ~60 lines of unnecessary trait definitions and boilerplate.
