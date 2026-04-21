## 2024-05-10 - [Rect Module Documentation]
**Confusion:** The 2D primitive rendering for rectangles was entirely undocumented, leading to confusion on what parameters it takes and what it does.
**Clarification:** Added a module-level doc `//!` explaining its use cases, and added docstrings for `draw_rect`, `fill_rect`, `draw_rounded_rect`, and `fill_rounded_rect` including parameters and executable doctests.
## 2024-05-11 - [Ellipse Rasterization]
**Confusion:** The `draw_ellipse` and `fill_ellipse` functions were missing documentation, leaving developers confused about their coordinate bounds handling and inner loop performance characteristics.
**Clarification:** Added complete module and function documentation emphasizing the bounds-free fast-path logic and performance characteristics. Supplied copy-pasteable examples.
## 2024-05-12 - [Scanline Rasterization Parameters]
**Confusion:** The scanline primitive functions (`draw_scanline_flat`, `draw_scanline_textured_perspective`, etc.) lacked explicit documentation for their numerous mathematical parameters (e.g. `dz_dx`, `dc_dx`, `gradients`), making it hard for developers extending the rasterizer to know what values to supply.
**Clarification:** Added comprehensive `# Arguments` lists for each of these core scanline rendering functions, and added `# Panics` and `# Errors` sections for related RenderTarget methods that lacked them.
