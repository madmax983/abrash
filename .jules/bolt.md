**[Performance Optimization: Inline Get Depth]**
**Learning:** Checking bounds and returning an `Option<f32>` on every pixel in an inner loop via `ZBuffer::get_depth` introduces significant overhead and prevents optimal compiler inlining.
**Action:** For performance critical inner loops (like SSAO scalar fallback), where bounds are already mathematically proven by clamping the loop ranges (e.g. `s_screen_x >= 0 && s_screen_x < width`), manually inline the depth check using `.as_slice()[idx]`. This bypasses the safe Option wrapper and yields massive speedups.
