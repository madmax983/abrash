## 2024-05-10 - [Rect Module Documentation]
**Confusion:** The 2D primitive rendering for rectangles was entirely undocumented, leading to confusion on what parameters it takes and what it does.
**Clarification:** Added a module-level doc `//!` explaining its use cases, and added docstrings for `draw_rect`, `fill_rect`, `draw_rounded_rect`, and `fill_rounded_rect` including parameters and executable doctests.
## 2024-05-11 - [Ellipse Rasterization]
**Confusion:** The `draw_ellipse` and `fill_ellipse` functions were missing documentation, leaving developers confused about their coordinate bounds handling and inner loop performance characteristics.
**Clarification:** Added complete module and function documentation emphasizing the bounds-free fast-path logic and performance characteristics. Supplied copy-pasteable examples.
## 2024-05-12 - [Line Rendering Documentation]
**Confusion:** The `rasterizer::line` module had no high-level explanation of how 3D lines interact with the Z-buffer and frustum, causing users to mistakenly try using it for 2D UI overlays, leading to projection panics.
**Clarification:** Added `//!` module documentation to `rasterizer::line` to explain the 3D pipeline constraints. Added executable doctests to `draw_line_3d` and `fill_triangle_wireframe` demonstrating proper Homogeneous Clip Space (W=1.0) setups.
## 2024-05-13 - [Procedural Texture Documentation]
**Confusion:** The `procedural` module lacked copy-pasteable doctests, leaving users confused about how to use its texture generation functions.
**Clarification:** Added executable examples demonstrating how to use `xor_pattern`, `grid_pattern`, `white_noise`, and `plasma` to generate and verify textures.
## 2024-05-14 - [Core and Render Undocumented Items]
**Confusion:** Several public structs (`Cylinder`), modules (math primitives), and thread-locals (`CPU_RENDERER_DRAW_LIST`) were lacking documentation, generating missing_docs warnings and leaving their use-cases unclear.
**Clarification:** Added appropriate module and struct level documentation to clarify their purpose in the larger 3D graphics pipeline context.
## 2024-05-18 - [Missing Documentation for Public Modules]
**Confusion:** Several experimental modules and internal fuzzing functions lacked proper documentation or were exposed to the public API unnecessarily, leading to a poorer `cargo doc` output and `clippy::missing_docs` / `clippy::doc_markdown` warnings.
**Clarification:** Added missing module-level `//!` and function-level `///` docs to `tunnel.rs`, `mandelbrot.rs`, `precipitation.rs`, `water_ripple.rs`, `kaleidoscope.rs`, and `modifiers.rs`. Also applied `#[doc(hidden)]` to `fuzz_load_obj` in `obj_loader_fuzz.rs` to keep the public manual clean. Fixed unescaped hex literals in doc comments by wrapping them in backticks to resolve `clippy::doc_markdown`.
## 2024-05-19 - [Skybox Documentation]
**Confusion:** The `skybox` module lacked executable doctests, leaving users confused about how to initialize the `Cubemap` and format matrices for `draw_skybox`.
**Clarification:** Added executable `## Examples` block to both the `Cubemap` struct and `draw_skybox` function to demonstrate proper initialization and rendering.

## 2024-05-19 - [TuiWindow Documentation]
**Confusion:** The `TuiWindow` struct in the platform module lacked documentation, which could cause confusion regarding its role as a terminal backend window manager.
**Clarification:** Added a struct-level documentation explaining that it manages a terminal backend using `ratatui` + `crossterm`.
## 2024-06-01 - [CameraState vs GpuInteractionController Confusion]
**Confusion:** When attempting to document the `abrash-gpu-render` methods (`adjust_zoom`, `update`, `apply_mouse_drag`), I initially assumed the struct implementing them was `CameraState` (a common name for this pattern). The tests failed because the type is actually `GpuInteractionController`.
**Clarification:** Corrected the doctests to instantiate `GpuInteractionController::new(&config)` instead of `CameraState`. Always verify the struct name in the `impl` block before writing doctests.
