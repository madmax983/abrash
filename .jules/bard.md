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
