**[WGPU Pipeline Extraction]**
**Learning:** In wgpu-based rendering code, embedding verbose `wgpu::BindGroupLayoutDescriptor` and `wgpu::RenderPipelineDescriptor` definitions directly inside `new()` constructors creates massive 'God Functions' with deep nesting.
**Action:** Extract these configurations into private, descriptively named helper functions (e.g., `create_gbuffer_pipeline`) to dramatically flatten the code and improve readability. When extracting `wgpu::RenderPipeline` creation logic, the `vertex_layouts` slice argument must explicitly declare lifetimes (e.g., `&[wgpu::VertexBufferLayout<'_>]`) to correctly satisfy the borrow checker.
**Fix Integer Overflows in draw_line_3d**
**Learning:** Fuzzing line-drawing functions with extreme floating-point numbers can cause intermediate error calculations to integer-overflow.
**Action:** Switched standard Bresenham error tracking variables (`dx`, `dy`, `err`, `e2`) to use `i64` inside `draw_line_3d` in `crates/abrash-render/src/rasterizer/line.rs` to allow safe calculation without geometric distortion or clipping.
