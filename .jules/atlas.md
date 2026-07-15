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

## [Argument Jungle Fix for Texture Rasterization]
**Tangle:** The `post_process` and `texture` rendering functions were suffering from the "Argument Jungle" anti-pattern. Functions like `draw_span_textured_gouraud_simd` and its scalar variants took upwards of 15 primitive arguments (`z_start`, `u_fix_start`, `du_fix`, `dr_dx`, etc.), leading to cognitive overload, brittle function signatures, and disorganized data flows.

**Blueprint:**
1.  **Extract Structs:** Created `TexSpanState` and `TexSpanStep` to bundle standard perspective texture span parameters. Created `GouraudSpanState` and `GouraudSpanStep` to bundle textured Gouraud shading span parameters.
2.  **Refactor Signatures:** Modified all `draw_span_*` scalar and SIMD functions in `crates/abrash-render/src/rasterizer/texture.rs` to accept these new configuration structs instead of loose arguments.
3.  **Update Callers:** Updated `draw_scanline_textured_perspective`, `draw_scanline_textured_gouraud`, and tile rasterizer logic to construct and pass the new state and step structs.

**Stability:** Improved clarity and lowered argument count below the cognitive limit. High cohesion is achieved by grouping related interpolation state variables together, making the low-level rendering API significantly cleaner and easier to maintain.

## [Unified Experimental Error Handling]
**Tangle:** Inconsistent error handling (mixing `String` and `&'static str`) across `experimental` modules like `lsystem`, `arboretum`, `jelly`, and `steganography`, creating an unpredictable API. Additionally, unreadable numeric literals were triggering `clippy` warnings, and the lack of `# Errors` documentation broke formatting standards.

**Blueprint:**
1.  **Introduce Central Error:** Created a unified `Error` enum in `crates/abrash-render/src/experimental/error.rs` to encapsulate all experimental failure modes (e.g., `CapacityExceeded`, `MeshIndexOutOfBounds`).
2.  **Refactor Modules:** Updated experimental modules to return `Result<T, crate::experimental::error::Error>` instead of primitive string errors. Addressed `clippy` warnings by adding `# Errors` documentation and formatting unreadable numeric literals.
3.  **Result:** Standardized error boundaries within the `experimental` module, improving maintainability and ensuring safe, idiomatic error propagation.

## [SIMD Alignment Crash Fix in TileRenderer]
**Tangle:** The `TileRenderer` implemented thread-local buffering for parallel rendering (`end_frame_into_slices`) using standard `Vec<u32>` and `Vec<f32>` arrays in `TILE_BUFFER`. However, standard vectors only guarantee alignment up to their element size (4 bytes). When this unaligned memory was passed down to the AVX2 `rasterize_scanline_simd` routine, it caused a hard `SIGSEGV` segmentation fault due to the `_mm256_store_ps` intrinsic requiring strict 32-byte alignment. This caused the test suite to silently crash.

**Blueprint:**
1. **Refactor Type:** Replaced `Vec<T>` with `AlignedBuffer<T>` inside the thread-local `TILE_BUFFER` in `crates/abrash-render/src/rasterizer/tile.rs`.
2. **Expose Resizing:** Added a `resize(&mut self, new_len, default_value)` method to the `AlignedBuffer` utility struct to emulate the `Vec` behavior required by the parallel iteration logic, while explicitly maintaining the 32-byte memory offset.

**Stability:** Resolves a critical threading SIGSEGV crash. `AlignedBuffer` guarantees that SIMD intrinsics perform safe aligned memory accesses without trapping on hardware boundaries.
**[Winit/TUI Backend Split]
**Tangle:** The `abrash` workspace examples directly relied on `winit`-specific types (`WindowContext` holding an `EventLoopWindowTarget`) and `WindowApp` traits defined inside `src/platform/winit.rs`. This meant compiling with only the `backend-tui` feature failed entirely because the unified trait was gated behind `winit` dependencies.
**Blueprint:** Abstracted `WindowApp`, `WindowContext`, and related event enums directly in the `platform::tui` module mirroring the `winit` types. Updated `mod.rs` to conditionally export `tui::*` or `winit::*` based on the active backend feature. Updated examples to conditionally import event types (`KeyEvent`, `ElementState`, `WindowEvent`, `Key`, `NamedKey`) based on the active backend feature.
**Stability:** Decouples `backend-tui` from `winit`, enabling standalone terminal UI builds of all CPU examples. Prevents cross-contamination of backend states while presenting a unified trait to caller applications.
**Verification:** Run `cargo check --no-default-features --features backend-tui` and ensure examples like `cube_3d` compile flawlessly.
## [Unified Experimental Error Handling]
**Tangle:** Inconsistent error handling (mixing `String` and `&'static str`) across `experimental` modules like `lsystem`, `arboretum`, `jelly`, and `steganography`, creating an unpredictable API.
**Blueprint:**
1.  **Introduce Central Error:** Created a unified `Error` enum in `crates/abrash-render/src/experimental/error.rs` to encapsulate all experimental failure modes (e.g., `CapacityExceeded`, `MeshIndexOutOfBounds`).
2.  **Refactor Modules:** Updated experimental modules to return `Result<T, crate::experimental::error::Error>` instead of primitive string errors.
3.  **Result:** Standardized error boundaries within the `experimental` module, improving maintainability and ensuring safe, idiomatic error propagation.
## [Doc Test Import Boundary Fix]
**Tangle:** The `TuiWindow` doc test in `src/platform/tui.rs` was attempting to import itself via `use abrash_render::platform::tui::TuiWindow;`. This fails compilation and violates the workspace structural boundary because the `platform` module is not defined in or exported from the `abrash-render` crate; it is defined in the root `abrash` facade crate.
**Blueprint:** Modified the doc test import to `use abrash::platform::tui::TuiWindow;`. This respects the workspace crate boundaries by resolving the module from the correct root crate, passing tests without introducing new dependencies or leaking internal structure.

## [Missing Test Feature Gates Fix]
**Tangle:** Several experimental/parallel post-processing and rendering tests in `crates/abrash-render/tests/` were failing to compile or resolve module paths under a bare `cargo test` run because they implicitly required the `nova` or `parallel` feature flags, but lacked the necessary `#![cfg(feature = "...")]` module attributes to exclude them during default builds.
**Blueprint:**
1.  **Isolate:** Added `#![cfg(feature = "nova")]` to `havoc_radial_blur_fuzz.rs`, `havoc_pixel_sort_proptest.rs`, and `havoc_directional_blur.rs`. Added `#![cfg(feature = "parallel")]` to `havoc_tile_ub_test.rs`.
2.  **Result:** Ensure `cargo test --workspace` does not fail due to unresolvable feature-gated imports on default builds.

**[Skybox and Skeleton Dependency Cycles]
**Tangle:** The `abrash-render` crate had a circular dependency between the `skybox` and `rasterizer` modules (`skybox` imported `rasterizer::texture::fill_quad_textured` while `rasterizer::reflection` imported `skybox::Cubemap`). Concurrently, the `abrash-skeletal` crate had a circular dependency between `pose` and `skeleton` (`pose` imported `Skeleton` for `Pose::from_bind` and `skeleton` imported `Pose`).
**Blueprint:**
1.  **Decoupled Skybox:** Extracted the `Cubemap` data structure down into `crates/abrash-core/src/texture.rs`. Re-exported it in `skybox.rs` via `pub use abrash_core::texture::Cubemap;` to maintain the public API, allowing `rasterizer::reflection` to depend on `abrash_core` instead of `skybox`.
2.  **Decoupled Skeleton:** Moved `Pose::from_bind(&skeleton)` into `Skeleton::bind_pose(&self)`, removing `crate::skeleton::Skeleton` from `pose.rs` and cleanly breaking the module cycle.
**Stability:** Enforced unidirectional module graphs, improving architecture health without breaking API consumers.
**Verification:** Tests build and pass seamlessly without circular `use` statements.
