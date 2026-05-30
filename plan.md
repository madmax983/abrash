1. **Define `TexSpanState` and `TexSpanStep` structs**:
   - Add these structs to `crates/abrash-render/src/rasterizer/texture.rs`.
   - `TexSpanState` should bundle `z`, `u_fix`, `v_fix`.
   - `TexSpanStep` should bundle `dz_dx`, `du_fix`, `dv_fix`.
2. **Define `GouraudSpanState` and `GouraudSpanStep` structs**:
   - Add these structs to `crates/abrash-render/src/rasterizer/texture.rs`.
   - `GouraudSpanState` should include `z`, `u_fix`, `v_fix`, `r_fix`, `g_fix`, `b_fix`.
   - `GouraudSpanStep` should include `dz_dx`, `du_fix`, `dv_fix`, `dr_dx`, `dg_dx`, `db_dx`.
3. **Refactor `draw_span_*` scalar functions**:
   - Update `draw_span_nearest`, `draw_span_bilinear` in `texture.rs` to take `TexSpanState` and `TexSpanStep`.
   - Update `draw_span_trilinear` in `texture.rs` to take `TexSpanState`, `TexSpanStep`, and a separate `lod: f32` parameter.
   - Update `draw_span_textured_gouraud_scalar` in `texture.rs` to take `GouraudSpanState` and `GouraudSpanStep`.
4. **Refactor `draw_span_*_simd` functions**:
   - Update `draw_span_nearest_simd`, `draw_span_bilinear_simd` in `texture.rs` to take `TexSpanState` and `TexSpanStep`.
   - Update `draw_span_trilinear_simd` in `texture.rs` to take `TexSpanState`, `TexSpanStep`, and `lod: f32`.
   - Update `draw_span_textured_gouraud_simd`, `draw_span_textured_gouraud_bilinear_simd` in `texture.rs` to take `GouraudSpanState` and `GouraudSpanStep`.
5. **Update callers in `texture.rs`**:
   - Update `draw_scanline_textured_perspective` and `draw_scanline_textured_gouraud` to use the new structs.
6. **Update callers in `tile.rs`**:
   - Update callers of `draw_span_nearest`, `draw_span_bilinear`, `draw_span_trilinear` and their `_simd` equivalents around lines 1241-1379 in `crates/abrash-render/src/rasterizer/tile.rs`.
7. **Verify compilation**:
   - Run `cargo check -p abrash-render --all-targets --all-features` to confirm the refactored function signatures and callers compile correctly.
8. **Complete pre-commit steps**:
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
9. **Submit the change**:
   - Submit the PR with the required PR title and description format.
