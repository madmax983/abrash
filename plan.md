1. **Define structs in `crates/abrash-render/src/rasterizer/texture.rs`**: Insert `TexSpanState` and `TexSpanStep` structs at the top of the file to hold basic texture mapping parameters (`z`, `u_fix`, `v_fix` for State; `dz_dx`, `du_fix`, `dv_fix` for Step). Also insert `GouraudSpanState` and `GouraudSpanStep` structs to hold textured gouraud mapping parameters (`z`, `u_fix`, `v_fix`, `r_fix`, `g_fix`, `b_fix` for State; `dz_dx`, `du_fix`, `dv_fix`, `dr_dx`, `dg_dx`, `db_dx` for Step). Use `derive(Clone, Copy)` and `pub(crate)` visibility.
2. **Refactor basic texturing functions in `crates/abrash-render/src/rasterizer/texture.rs`**: Modify `draw_span_nearest`, `draw_span_nearest_simd`, `draw_span_bilinear`, `draw_span_bilinear_simd`, `draw_span_trilinear`, and `draw_span_trilinear_simd` to accept `mut state: TexSpanState` and `step: &TexSpanStep` instead of loose parameters. Keep `lod` as a separate parameter for trilinear functions.
3. **Refactor gouraud texturing functions in `crates/abrash-render/src/rasterizer/texture.rs`**: Modify `draw_span_textured_gouraud_simd`, `draw_span_textured_gouraud_bilinear_simd`, and `draw_span_textured_gouraud_scalar` to accept `mut state: GouraudSpanState` and `step: &GouraudSpanStep` instead of loose parameters.
4. **Update call sites in `crates/abrash-render/src/rasterizer/texture.rs`**: Update all call sites inside this file that invoke the refactored functions to correctly instantiate and pass the new state and step structs based on line numbers 874, 1070, 1494, 1507, 1521, 1542, 1555, 1569, 1603, 1617, 1632, 2799, 2819, 3092, 3911, 4434, 4453, 4472, 4499, 5537.
5. **Update call sites in `crates/abrash-render/src/rasterizer/tile.rs`**: Update the call sites of `draw_span_nearest`, `draw_span_nearest_simd`, `draw_span_bilinear`, `draw_span_bilinear_simd`, `draw_span_trilinear`, and `draw_span_trilinear_simd` between lines 1241 and 1379 to instantiate and pass the new structs.
6. **Update call sites in tests**: Update the tests in `crates/abrash-render/src/rasterizer/havoc_proptests.rs` (lines 29, 57) and `crates/abrash-render/src/rasterizer/texture_havoc_test.rs` (lines 23, 44, 66) to construct and pass `TexSpanState` and `TexSpanStep`.
7. **Verify code changes**: Run `cargo check --all-targets --all-features`, `cargo test --all-features`, and `cargo fmt --all` to ensure the codebase compiles, tests pass, and code is formatted correctly.
8. **Record journal entry**: Add the following entry into `.jules/atlas.md`:
```
## [Argument Jungle Fix for Texture Rasterization]
**Tangle:** The `draw_span_` basic and gouraud texturing functions in `crates/abrash-render/src/rasterizer/texture.rs` were suffering from the "Argument Jungle" anti-pattern, taking upwards of 15 parameters (`z`, `dz_dx`, `u_fix`, `v_fix`, `du_fix`, `dv_fix`, `r_fix`, `g_fix`, `b_fix`, `dr_dx`, `dg_dx`, `db_dx`).
**Blueprint:**
1. **Extract Structs**: Created `TexSpanState` and `TexSpanStep` for basic texturing parameters, and `GouraudSpanState` and `GouraudSpanStep` for textured gouraud parameters.
2. **Refactor Signatures**: Modified all scalar and SIMD texturing functions to accept these structs instead of loose parameters.
3. **Update Callers**: Updated `texture.rs`, `tile.rs`, and tests to construct and pass the new structs.
**Stability**: Improved clarity and lowered argument count below the cognitive limit, making the low-level rendering API cleaner and easier to maintain.
```
9. **Complete pre-commit steps**: Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
10. **Submit the changes**: Submit the PR titled "🗺️ Atlas: [Argument Jungle Fix for Texture Rasterization]" using the `submit` tool.
