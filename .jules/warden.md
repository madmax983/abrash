**2024-05-24 - [Fix buffer overflow in 3D pipeline]**
**Threat:** Out-of-bounds write in `draw_scanline_flat` when `Framebuffer` and `ZBuffer` dimensions mismatch. The optimization using `unsafe { get_unchecked_mut }` relied on caller guarantees that were not enforced.
**Defense:** Added runtime assertions `assert_eq!(fb.width(), zb.width())` and `assert_eq!(fb.height(), zb.height())` in `fill_triangle_3d` and `fill_triangle_gouraud`.

**2024-05-24 - [Fix FFI Type Confusion in Windows Platform]**
**Threat:** Invalid pointer cast in `src/platform/win32.rs`. A UTF-8 string pointer (`*const u8`) was cast to `*const u16` and passed to `CreateWindowExW`, causing Undefined Behavior and garbage class names.
**Defense:** Implemented correct UTF-16 encoding using `encode_utf16().chain(once(0))` and updated function signatures to accept valid `&[u16]` slices.
