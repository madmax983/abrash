**2024-05-24 - [Fix buffer overflow in 3D pipeline]**
**Threat:** Out-of-bounds write in `draw_scanline_flat` when `Framebuffer` and `ZBuffer` dimensions mismatch. The optimization using `unsafe { get_unchecked_mut }` relied on caller guarantees that were not enforced.
**Defense:** Added runtime assertions `assert_eq!(fb.width(), zb.width())` and `assert_eq!(fb.height(), zb.height())` in `fill_triangle_3d` and `fill_triangle_gouraud`.

**2024-05-25 - [Fix integer overflows in rasterizer]**
**Threat:** Multiple integer overflows identified:
1. `i32::MIN` negation panics in `draw_scanline_flat` (DoS).
2. `i32` subtraction overflow in vertex interpolation causing incorrect gradients or wrapping artifacts.
3. `usize` overflow in `Framebuffer` size calculation on 32-bit systems allowing buffer under-allocation.
**Defense:**
1. Used `-(x as f32)` instead of `(-x) as f32`.
2. Cast coordinates to `f32` before subtracting during interpolation setup.
3. Added `checked_mul` validation in buffer constructors.
