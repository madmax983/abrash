1. **Define Configuration Structs**:
   - Create `TexSpanState` struct to hold state parameters like `z`, `u_fix`, `v_fix`.
   - Create `TexSpanStep` struct to hold step parameters like `dz_dx`, `du_fix`, `dv_fix`.
   - Create `GouraudSpanState` struct to hold color state parameters `r_fix`, `g_fix`, `b_fix`.
   - Create `GouraudSpanStep` struct to hold color step parameters `dr_dx`, `dg_dx`, `db_dx`.
   - Add these structs to `crates/abrash-render/src/rasterizer/texture.rs`.

2. **Refactor Texture Span Functions**:
   - Modify `draw_span_nearest` to use `TexSpanState` and `TexSpanStep`.
   - Modify `draw_span_nearest_simd` to use `TexSpanState` and `TexSpanStep`.
   - Modify `draw_span_bilinear` to use `TexSpanState` and `TexSpanStep`.
   - Modify `draw_span_bilinear_simd` to use `TexSpanState` and `TexSpanStep`.
   - Modify `draw_span_trilinear` to use `TexSpanState` and `TexSpanStep`.
   - Modify `draw_span_trilinear_simd` to use `TexSpanState` and `TexSpanStep`.

3. **Refactor Textured Gouraud Span Functions**:
   - Modify `draw_span_textured_gouraud_scalar` to use `TexSpanState`, `TexSpanStep`, `GouraudSpanState`, and `GouraudSpanStep`.
   - Modify `draw_span_textured_gouraud_simd` to use `TexSpanState`, `TexSpanStep`, `GouraudSpanState`, and `GouraudSpanStep`.
   - Modify `draw_span_textured_gouraud_bilinear_simd` to use `TexSpanState`, `TexSpanStep`, `GouraudSpanState`, and `GouraudSpanStep`.

4. **Update Callers**:
   - Update `draw_scanline_textured_perspective` and `draw_scanline_textured_gouraud` to construct and pass the new structs.
   - Update any other callers in `texture.rs`.
   - Check and update `crates/abrash-render/src/rasterizer/tile.rs` where these functions are called.
   - Fix compilation errors.

5. **Complete pre commit steps**:
   - Run `cargo fmt`, `cargo clippy`, and `cargo test` to ensure proper testing, verifications, reviews and reflections are done.

6. **Submit PR**:
   - Use `submit` to push the changes.
