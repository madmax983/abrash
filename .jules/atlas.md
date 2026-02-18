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

## [Centralized Utilities]
**Tangle:** The `XorShift32` RNG and luminance calculation logic were duplicated across `particles.rs`, `vhs.rs`, `procedural.rs`, `ascii.rs`, and `post_process.rs`. This violated DRY and created maintenance burden where fixes to one implementation wouldn't propagate to others.

**Blueprint:**
1.  **Extract:** Created `src/utils.rs` to house `XorShift32` and `pixel_luminance`.
2.  **Refactor:** Updated all dependent modules to use the centralized utilities.
3.  **Result:** High cohesion for utility logic; Reduced code duplication.

## [Consolidated Procedural Generation]
**Tangle:** Procedural logic was fragmented: `procedural.rs` handled textures, while `experimental/arboretum.rs` (L-Systems) and `experimental/procedural_mesh.rs` (Terrain) handled meshes. This created low cohesion and scattered related functionality.
**Blueprint:**
1.  **Group:** Created `src/procedural/` directory.
2.  **Relocate:** Moved `arboretum.rs` to `procedural/l_system.rs` and `procedural_mesh.rs` to `procedural/terrain.rs`.
3.  **Unify:** Created `src/procedural/mod.rs` to re-export all submodules, providing a single entry point `abrash::procedural`.
**Stability:** High cohesion for procedural generation. `experimental` is now reserved for truly experimental/unstable features (like jelly physics).
