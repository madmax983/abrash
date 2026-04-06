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

## [Argument Jungle Fix for DoF and SSAO]
**Tangle:** The `apply_ssao` and `apply_depth_of_field` functions were suffering from the "Argument Jungle" anti-pattern. `apply_ssao` took 6 arguments (including 3 configuration floats) and `apply_depth_of_field` took 5 arguments (including 3 configuration values), making it difficult to maintain and read.
**Blueprint:**
1.  **Extract:** Created `SsaoConfig` and `DepthOfFieldConfig` structs to encapsulate these parameters.
2.  **Refactor:** Updated function signatures to take a reference to the respective config struct. Updated all callers (examples, tests, doc comments) to instantiate and pass the new structs.
3.  **Result:** Lowered argument count, cohesive configurations for these post-processing effects, easier to extend.

## [Removing Windows-Sys OS Coupling]
**Tangle:** The project had a `backend-win32` platform feature utilizing unsafe FFI via `windows-sys` for direct Win32/GDI execution. Concurrently, it had a `backend-winit` feature providing safe, cross-platform functionality via `winit` and `softbuffer`. Several examples (`swirl_demo`, `vignette_demo`, `tilt_shift_demo`, `edge_glow_demo`) explicitly relied on the unsafe `Win32Window` abstraction. This duplicated windowing logic, bound those examples strictly to Windows environments, and caused workspace failures on Unix/Linux systems when developers ran workspace tests.
**Blueprint:**
1.  **Cut Obsolete Abstraction:** Deleted `src/platform/win32.rs` entirely, dropping the custom Win32 fallback implementation.
2.  **Prune Dependency Graph:** Removed `windows-sys` and the `backend-win32` feature flag from `Cargo.toml`.
3.  **Modernize Call Sites:** Migrated the aforementioned examples to rely directly on `winit`'s standard event-driven approach by mapping their procedural update loops to `WindowApp` implementations.
**[Optional RT Pass Instantiation]
**Tangle:** The `RtShadowPass` and `RtReflectionPass` WGPU shader module creations were blindly failing the test suite on runner environments that lacked the `wgpu_ray_query` extension feature. A test failure propagated because we did not guard the pass instantiation at runtime. Additionally, the WGPU objects were held in fields marked as `pub(crate)`, emitting dead code warnings.
**Blueprint:** Encapsulated WGPU objects into private underscore-prefixed fields (e.g. `_pipeline`). Changed `rt_shadow_pass` within `GpuRenderer` to an `Option<RtShadowPass>` and conditionally instantiated it only if `device.features().contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)` returns true at runtime, preventing panics.**

## [Filter Parameterization Fix for Chromatic Aberration]
**Tangle:** The `apply_chromatic_aberration` function suffered from the "Argument Jungle" structural smell, accepting a loose primitive parameter (`offset`). This created a fragmented API alongside the other configured post-processing filters, making future extensions difficult.

**Blueprint:**
1.  **Introduce Configs:** Created `ChromaticAberrationConfig` struct in `crates/abrash-render/src/post_process/filters.rs`.
2.  **Refactor Signatures:** Modified `apply_chromatic_aberration` to accept a reference to this new configuration struct.
3.  **Update Callers:** Updated doc tests, unit tests, integration tests, and benchmarks to instantiate the required configuration struct.

**Stability:** Improved high cohesion by standardizing the effect parameterization with the rest of the post-processing module. Lowered coupling between the caller and the specific internal parameters of the post-processing effect, making the public API cleaner and more extensible.

## [Decoupling Raycaster from Render Crate]
**Tangle:** The `abrash-render` crate depended on the `abrash-raycast` crate solely to host the raycasting rendering implementation modules (`bsp.rs`, `bsp_lighting.rs`, `hybrid.rs`). This artificially coupled a specialized rendering logic to the core software rasterization pipeline of `abrash-render`.
**Blueprint:**
1.  **Relocate:** Moved the raycaster implementation modules from `crates/abrash-render/src/raycaster` to `crates/abrash-raycast/src/renderer`.
2.  **Prune Dependency:** Removed `abrash-raycast` dependency from `abrash-render`'s `Cargo.toml`.
3.  **Facade:** Updated the root workspace facade (`src/lib.rs`) to re-export the relocated module via `pub use abrash_raycast::renderer as raycaster;`, preserving the public API while severing the structural dependency.

## [Decoupling Scene from TileRenderer]
**Tangle:** The `Scene` struct inside `crates/abrash-render/src/scene.rs` defined a `render()` method that directly passed `&mut TileRenderer`. This bypassed the stable `render_api`'s `CpuRenderer` completely, creating a redundant path for rendering geometries and coupling the high-level scene graph directly to low-level rasterization details.
**Blueprint:**
1. **Remove Coupling:** Deleted the `render` method from `Scene` and removed all references to `TileRenderer` inside the module.
2. **Standardize Workflows:** Reoriented dependent examples (like `directional_blur_demo.rs`) and test cases to use `Scene::extract()` to produce a `DrawList` and then dispatch it via `CpuRenderer::execute_draw_list`.
**Stability:** Restored unidirectional dependencies (`Scene` -> `DrawList` -> `CpuRenderer` -> `TileRenderer`) matching the engine's core graphics pipeline philosophy.
