**[WGPU Pipeline Extraction]**
**Learning:** In wgpu-based rendering code, embedding verbose `wgpu::BindGroupLayoutDescriptor` and `wgpu::RenderPipelineDescriptor` definitions directly inside `new()` constructors creates massive 'God Functions' with deep nesting.
**Action:** Extract these configurations into private, descriptively named helper functions (e.g., `create_gbuffer_pipeline`) to dramatically flatten the code and improve readability. When extracting `wgpu::RenderPipeline` creation logic, the `vertex_layouts` slice argument must explicitly declare lifetimes (e.g., `&[wgpu::VertexBufferLayout<'_>]`) to correctly satisfy the borrow checker.
**Extract Mesh Validation**
**Learning:** Duplicated loops for boundary validation inside API wrappers (like `create_mesh`, `create_mesh_owned`, and `update_mesh`) are a form of the "Primitive Cluster" code smell.
**Action:** Always extract these identical iteration blocks into a single private helper function (e.g., `validate_mesh_indices`). This prevents subtle divergence, dramatically reduces the size of the calling functions, and clearly documents the intent of the block.

**2025-05-18 - Revert Framebuffer and ZBuffer clear_rect optimization**
**Learning:** Benchmarks confirmed that the speculative `clear_rect` optimization using explicit index calculations with `unsafe { get_unchecked_mut() }` was actually slower than standard safe iterator-based chunking (`.chunks_exact_mut()`) for 2D region fills, particularly on larger resolutions. Safe iterator chunking allows the compiler to better vectorize and optimize the fill operation.
**Action:** Reverted `Framebuffer::clear_rect` and `ZBuffer::clear_rect` to use the standard safe `chunks_exact_mut()` approach without bypassing bounds checks in the inner loop, restoring safety and resolving performance regressions across various resolutions.
