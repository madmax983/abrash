**[WGPU Pipeline Extraction]**
**Learning:** In wgpu-based rendering code, embedding verbose `wgpu::BindGroupLayoutDescriptor` and `wgpu::RenderPipelineDescriptor` definitions directly inside `new()` constructors creates massive 'God Functions' with deep nesting.
**Action:** Extract these configurations into private, descriptively named helper functions (e.g., `create_gbuffer_pipeline`) to dramatically flatten the code and improve readability. When extracting `wgpu::RenderPipeline` creation logic, the `vertex_layouts` slice argument must explicitly declare lifetimes (e.g., `&[wgpu::VertexBufferLayout<'_>]`) to correctly satisfy the borrow checker.
**Extract Mesh Validation**
**Learning:** Duplicated loops for boundary validation inside API wrappers (like `create_mesh`, `create_mesh_owned`, and `update_mesh`) are a form of the "Primitive Cluster" code smell.
**Action:** Always extract these identical iteration blocks into a single private helper function (e.g., `validate_mesh_indices`). This prevents subtle divergence, dramatically reduces the size of the calling functions, and clearly documents the intent of the block.
**[Extracted Inner Logic for Drawing]**
**Learning:** Functions like `execute_draw_list_owned` and `execute_draw_list_into` contained identical internal rendering logic causing duplication and reducing maintainability.
**Action:** Extract this identical rendering logic into a single private helper function (e.g., `execute_draw_list_inner`) that both functions can delegate to. This adheres to DRY principles and eliminates duplication.
