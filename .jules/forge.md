**[WGPU Pipeline Extraction]**
**Learning:** In wgpu-based rendering code, embedding verbose `wgpu::BindGroupLayoutDescriptor` and `wgpu::RenderPipelineDescriptor` definitions directly inside `new()` constructors creates massive 'God Functions' with deep nesting.
**Action:** Extract these configurations into private, descriptively named helper functions (e.g., `create_gbuffer_pipeline`) to dramatically flatten the code and improve readability. When extracting `wgpu::RenderPipeline` creation logic, the `vertex_layouts` slice argument must explicitly declare lifetimes (e.g., `&[wgpu::VertexBufferLayout<'_>]`) to correctly satisfy the borrow checker.
**Extract Mesh Validation**
**Learning:** Duplicated loops for boundary validation inside API wrappers (like `create_mesh`, `create_mesh_owned`, and `update_mesh`) are a form of the "Primitive Cluster" code smell.
**Action:** Always extract these identical iteration blocks into a single private helper function (e.g., `validate_mesh_indices`). This prevents subtle divergence, dramatically reduces the size of the calling functions, and clearly documents the intent of the block.
**[Extracted Inner Logic for Drawing]**
**Learning:** Functions like `execute_draw_list_owned` and `execute_draw_list_into` contained identical internal rendering logic causing duplication and reducing maintainability.
**Action:** Extract this identical rendering logic into a single private helper function (e.g., `execute_draw_list_inner`) that both functions can delegate to. This adheres to DRY principles and eliminates duplication.
**[Extract Shared EdgeWalker Init]**\n**Learning:** When implementing multiple struct variants that share similar primitive logic (e.g., `EdgeWalker` implementations across different rasterizers like Phong, Pbr, or Gouraud), screen-space delta math and setup routines (`inv_h`, `dx_dy`, `dz_dy`) are frequently duplicated in their `new()` constructors.\n**Action:** Extract this common 2D coordinate initialization into a shared helper struct (e.g., `BaseEdgeDelta`) in a common module (like `core.rs`) effectively eliminating these "Primitive Cluster" and "God Function" anti-patterns.

**[Extract BaseTextureGradients]**
**Learning:** In `abrash-render`, duplicated screen-space delta math and setup routines (e.g., computing `ux`, `vy`, `inv_nz`, `dz_dx`) across rasterizer struct initializers like `PerspectiveTextureGradients`, `NormalMapGradients`, and `TexturedGouraudGradients` constitute a 'Primitive Cluster' and "Copy-Paste" anti-pattern.
**Action:** Extract these shared computations into a common helper struct (e.g., `BaseTextureGradients` in `core.rs`) and use it in the respective `new` functions. This DRYs the codebase, reduces function sizes, and improves clarity.
