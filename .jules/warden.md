**2024-05-24 - [Fix buffer overflow in 3D pipeline]**
**Threat:** Out-of-bounds write in `draw_scanline_flat` when `Framebuffer` and `ZBuffer` dimensions mismatch. The optimization using `unsafe { get_unchecked_mut }` relied on caller guarantees that were not enforced.
**Defense:** Added runtime assertions `assert_eq!(fb.width(), zb.width())` and `assert_eq!(fb.height(), zb.height())` in `fill_triangle_3d` and `fill_triangle_gouraud`.

**2024-05-24 - [Fix FFI Type Confusion in Windows Platform]**
**Threat:** Invalid pointer cast in `src/platform/win32.rs`. A UTF-8 string pointer (`*const u8`) was cast to `*const u16` and passed to `CreateWindowExW`, causing Undefined Behavior and garbage class names.
**Defense:** Implemented correct UTF-16 encoding using `encode_utf16().chain(once(0))` and updated function signatures to accept valid `&[u16]` slices.

**2024-05-25 - [Fix Integer Overflow in Framebuffer Dimensions]**
**Threat:** Integer overflow when `Framebuffer` width/height exceeds `i32::MAX`. The rasterizer casts `u32` dimensions to `i32` for coordinate calculations, leading to negative values and potential out-of-bounds writes in `unsafe` blocks.
**Defense:** Enforced `width <= i32::MAX` and `height <= i32::MAX` in `Framebuffer::new` and `ZBuffer::new`, returning an error if exceeded. Added regression test `tests/security_limits.rs`.

**2024-05-27 - [Fix Integer Overflow in Texture Allocation]**
**Threat:** Integer overflow when calculating texture buffer size `(width * height)` for large dimensions (e.g., 65536x65536). This resulted in a small allocation (wrapping to 0) but valid dimensions, causing heap buffer overflow during pixel access.
**Defense:** Changed `Texture::new` to return `Result`, enforced `width * height` fits in `u32` and `usize`, and added regression test `tests/security_texture_overflow.rs`.

**2024-05-28 - [Harden OBJ Loader and Window Creation]**
**Threat:** Potential Undefined Behavior in `obj_loader.rs` due to unnecessary `unsafe` indexing, and Integer Overflow in `Win32Window::new` / `TuiWindow::new` causing invalid window dimensions.
**Defense:** Replaced `unsafe` blocks with safe indexing in `obj_loader.rs`. Added bounds checks (`width > i32::MAX`, `height > i32::MAX`, zero checks) in `Win32Window::new` and `TuiWindow::new`. Added regression test `tests/security_obj_loader.rs`.

**2026-02-18 - [Hardened Rasterizer against OOB Scanlines]**
**Threat:** Potential Buffer Overflow / Undefined Behavior in scanline rasterizers. The functions `draw_scanline_*` in `gouraud.rs`, `texture.rs`, and `phong.rs` accepted a `y` coordinate (i32) and used it to calculate an array offset `(y * width)` without validating if `y` was within the framebuffer's vertical bounds. A negative or excessively large `y` could lead to an invalid offset and subsequent out-of-bounds write via `get_unchecked_mut`.
**Defense:** Added explicit bounds checks (`if y < 0 || y >= height { return; }`) at the start of all public and internal scanline drawing functions.

**2026-02-23 - [Harden Chromatic Aberration Buffer & Math Layout]**
**Threat:** Uninitialized memory access in `apply_chromatic_aberration` due to `unsafe { set_len(width) }` usage on a reused thread-local buffer. Also, potential Undefined Behavior in `transform_points_avx2` if `(Vec3, f32)` tuple layout is not packed (16 bytes).
**Defense:** Replaced `set_len` with safe `resize(width, 0)` in `filters.rs`. Added runtime `debug_assert!` in `math.rs` to enforce `(Vec3, f32)` layout assumptions (size 16, align 4) before unsafe SIMD operations.

**2026-02-24 - [Fix Integer Overflow in SIMD Bloom Filter]**
**Threat:** Integer overflow in `blend_additive_avx2` when `intensity >= 1.0`. The multiplication of pixel values (0-255) by scale factor (256+) caused overflow in signed 16-bit arithmetic, leading to incorrect colors (wrapping/clamping to 0 instead of saturation).
**Defense:** Updated `blend_additive_avx2` to use `_mm256_mulhi_epu16` for correct 32-bit intermediate multiplication logic, and used `_mm256_adds_epu16` with explicit clamping to prevent wrap-around before packing. Also enforced clamping of intensity input.
