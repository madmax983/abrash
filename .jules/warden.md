**2024-05-24 - [Fix buffer overflow in 3D pipeline]**
**Threat:** Out-of-bounds write in `draw_scanline_flat` when `Framebuffer` and `ZBuffer` dimensions mismatch. The optimization using `unsafe { get_unchecked_mut }` relied on caller guarantees that were not enforced.
**Defense:** Added runtime assertions `assert_eq!(fb.width(), zb.width())` and `assert_eq!(fb.height(), zb.height())` in `fill_triangle_3d` and `fill_triangle_gouraud`.
