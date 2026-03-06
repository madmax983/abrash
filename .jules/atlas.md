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

## [Relocate TileRenderer to Rasterizer]
**Tangle:** `src/tile_renderer.rs` was a top-level module in `src/`, polluting the root namespace and violating the principle that `rasterizer` should own all rasterization strategies. It was also tightly coupled with `rasterizer` internals, importing private/crate-public items from `src/rasterizer/texture.rs`.

**Blueprint:**
1.  **Relocate:** Moved `src/tile_renderer.rs` to `src/rasterizer/tile.rs`.
2.  **Encapsulate:** Exposed `TileRenderer` via `src/rasterizer/mod.rs` (`pub mod tile`), unifying it with other rasterization strategies (scanline, etc.).
3.  **Refactor:** Updated imports in `src/scene.rs` and tests to use `abrash::rasterizer::TileRenderer`.

**Result:** Improved cohesion within the `rasterizer` module. Reduced surface area of `src/lib.rs`. Stronger encapsulation of rasterizer internals.
## [Config Structs for Effect Parameterization]
**Tangle:** The `apply_ssao_scalar` and `apply_god_rays` functions were suffering from the "Argument Jungle" anti-pattern. `apply_ssao_scalar` took 9 arguments, including redundant `width` and `height` that could be extracted from existing arguments. `apply_god_rays` took 8 arguments, making it difficult to maintain and read.

**Blueprint:**
1.  **Eliminate Redundancy:** Removed `width` and `height` from `apply_ssao_scalar`, fetching them internally from `ZBuffer::width()` and `ZBuffer::height()`.
2.  **Config Struct:** Created `GodRaysConfig` struct to group 7 parameter arguments into a single cohesive configuration block.

**Stability:** Improved clarity and lowered argument count below the cognitive (and linter) limit of 7. Highly cohesive configurations make these effects easier to call in examples.

## [Filter Parameterization]
**Tangle:** The `apply_color_adjust` and `apply_vignette` functions suffered from the "Argument Jungle" structural smell, accepting multiple loose primitive parameters (e.g., `brightness`, `contrast`, `intensity`, `roundness`). This created a fragmented API and made future extensions difficult without breaking the function signatures.

**Blueprint:**
1.  **Introduce Configs:** Created `ColorAdjustConfig` and `VignetteConfig` structs in `src/post_process/filters.rs`.
2.  **Refactor Signatures:** Modified `apply_color_adjust` and `apply_vignette` to accept references to these new configuration structs.
3.  **Update Callers:** Updated doc tests, unit tests, integration tests, and benchmarks to instantiate the required configuration structs.

**Stability:** Improved high cohesion by grouping related effect parameters together. Lowered coupling between the caller and the specific internal parameters of the post-processing effects, making the public API cleaner and more extensible.
