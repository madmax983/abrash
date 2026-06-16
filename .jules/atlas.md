## [Argument Jungle Fix for Texture Rasterization]
**Tangle:** The `post_process` and `texture` rendering functions were suffering from the "Argument Jungle" anti-pattern. Functions like `draw_span_textured_gouraud_simd` and its scalar variants took upwards of 15 primitive arguments (`z_start`, `u_fix_start`, `du_fix`, `dr_dx`, etc.), leading to cognitive overload, brittle function signatures, and disorganized data flows.

**Blueprint:**
1.  **Extract Structs:** Created `TexSpanState` and `TexSpanStep` to bundle standard perspective texture span parameters. Created `GouraudSpanState` and `GouraudSpanStep` to bundle textured Gouraud shading span parameters.
2.  **Refactor Signatures:** Modified all `draw_span_*` scalar and SIMD functions in `crates/abrash-render/src/rasterizer/texture.rs` to accept these new configuration structs instead of loose arguments.
3.  **Update Callers:** Updated `draw_scanline_textured_perspective`, `draw_scanline_textured_gouraud`, and tile rasterizer logic to construct and pass the new state and step structs.

**Stability:** Improved clarity and lowered argument count below the cognitive limit. High cohesion is achieved by grouping related interpolation state variables together, making the low-level rendering API significantly cleaner and easier to maintain.
