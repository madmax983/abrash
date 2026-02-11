# Atlas's Journal

## [Decoupling 3D Rasterization from Pipeline]
**Tangle:** The `pipeline` module was acting as a "God Module" for 3D rendering, handling both high-level lighting/shading logic and low-level scanline rasterization. This coupled the concept of a "Pipeline" tightly to the specific software rasterization implementation, preventing the `rasterizer` module from being the single source of truth for pixel drawing. Additionally, `projection` logic was trapped in `pipeline`, creating potential circular dependencies if `rasterizer` needed to project vertices.

**Blueprint:**
1.  **Relocate Projection:** Move `ScreenPoint` and `project_to_screen` to `math` (the lowest common denominator).
2.  **Unify Rasterization:** Move 3D triangle filling and scanline logic from `pipeline` to `rasterizer`.
3.  **Thin Pipeline:** `pipeline` becomes a coordinator that handles lighting and shading, then delegates drawing to `rasterizer`.

**Stability:** Reduces coupling between rendering stages. `rasterizer` becomes cohesive (2D + 3D drawing). `pipeline` becomes focused on "Shader" logic. acyclic dependency graph is preserved.

## [Unified Mesh Interop]
**Tangle:** The `Mesh` struct (CPU rasterizer) and `GpuVertex` (GPU rasterizer) were disconnected, forcing users and examples to manually bridge the gap with custom conversion logic. This created a "Shotgun" smell where changes to one system didn't propagate to the other, and duplicated conversion code appeared in examples.

**Blueprint:**
1.  **Introduce Bridge:** Added `mesh_to_gpu` in `src/gpu_render.rs` to standardise the conversion from `Mesh` to `GpuVertex` + indices.
2.  **Verify with Example:** Created `examples/gpu_obj.rs` to demonstrate loading an OBJ (CPU) and rendering it via the GPU backend using the new bridge.
3.  **Result:** High cohesion in the `gpu_render` compatibility layer; Low coupling between core mesh logic and GPU implementation.

## [Centralized Color Utilities]
**Tangle:** Color manipulation logic (packing/unpacking u32, luminance calculation, blending) was scattered across `rasterizer`, `texture`, `post_process`, and `experimental` modules. This violated DRY and made it difficult to change color representation or add new color features without editing multiple files.

**Blueprint:**
1.  **Extract Module:** Created `src/color.rs` as a central utility module for all color operations.
2.  **Consolidate Logic:** Moved `pack_color`, `unpack_color`, `luminance`, `blend_swar`, and `from_vec3` logic into `color.rs`.
3.  **Refactor Dependencies:** Updated all consumers (`rasterizer`, `texture`, etc.) to depend on `color.rs`, making it a leaf node in the dependency graph (along with `math`).

**Stability:** High cohesion for color logic. `rasterizer` and `texture` are now focused on their core responsibilities, delegating color math to `color.rs`.
