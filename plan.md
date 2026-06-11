1.  **Refactor `draw_horizontal_line_unchecked` Optimization**
    *   In `crates/abrash-render/src/rasterizer/rect.rs`, read `draw_horizontal_line_unchecked`. It includes a check `if start_idx > end_idx || end_idx >= fb.as_mut_slice().len()` which performs safe boundary assertions before using `get_unchecked_mut`.
    *   Since `is_on_screen` guarantees boundary safety, this inner explicit `slice.len()` bounds check is redundant. Remove `|| end_idx >= fb.as_mut_slice().len()` to streamline execution further.

2.  **Refactor `draw_vertical_line_unchecked` Optimization**
    *   In `crates/abrash-render/src/rasterizer/rect.rs`, modify `draw_vertical_line_unchecked`. It includes an explicit check `if idx >= slice.len() || (y1 as usize * w + x as usize) >= slice.len() { return; }`.
    *   Since the calling context already validated `is_on_screen`, this inner safe boundary check is entirely redundant. Remove the `if` check completely to maximize performance within the unsafe logic.

3.  **Run Benchmarks**
    *   Run `cargo bench --bench rect_bench --features nova` to confirm measurable performance improvements on `draw_rect_100` and `draw_rounded_rect_100`.

4.  **Complete pre commit steps**
    *   Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done (e.g., run `cargo test` and `cargo clippy`).

5.  **Submit Changes**
    *   Commit changes using the `⚡ Bolt: [performance improvement]` format and matching PR description details.
