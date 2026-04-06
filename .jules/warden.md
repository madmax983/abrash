## 2024-05-18 - Out-of-Bounds Memory Write in Line Rasterizer
**Threat:** Missing dimension matching check in `draw_line_3d` in `crates/abrash-render/src/rasterizer/line.rs` could lead to OOB memory writes if the provided `ZBuffer` is smaller than the `Framebuffer`. This is a serious memory safety risk because it uses `get_unchecked_mut` to write into the `ZBuffer`.
**Defense:** Added `assert_same_dimensions(fb, zb);` at the top of the function to ensure the `Framebuffer` and `ZBuffer` are the exact same size before any unsafe memory operations occur, which enforces a safe boundary check prior to processing.

## 2024-05-18 - Integer Overflow Denial of Service in Circle Rasterizer
**Threat:** Unbounded inputs to the `radius` parameter in `draw_circle` and `fill_circle` in `crates/abrash-render/src/rasterizer/circle.rs` caused an integer overflow leading to panics when calculating the Bresenham decision parameter (`d = 3 - 2 * radius`). Furthermore, even without an overflow panic, excessively large radius inputs could hang the application in a DoS attack because the rasterizer attempts to loop up to `radius` times.
**Defense:** Hardened the input validation for circle rasterization by bounding the `radius` parameter to a maximum of 16384 (`radius = radius.min(16384)`). This effectively eliminates the integer overflow vector while mitigating the DoS loop iteration threat.
