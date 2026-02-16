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

**2026-02-07 - [Harden Texture Access and Parallel Rendering]**
**Threat:**
1. Out-of-bounds memory access (UB) in SIMD rasterizers when `Texture` dimensions do not match pixel buffer size (e.g. manually constructed malicious texture).
2. Unsound `Send`/`Sync` implementation for `SendPtr<T>` allowing non-thread-safe types to be shared/sent across threads in parallel renderer.
**Defense:**
1. Added runtime assertions `assert!(pixels.len() >= width * height)` in all texture-based rasterization paths (`draw_scanline_*`, `fill_triangle_*`).
2. Constrained `SendPtr<T>` to require `T: Send` for both `Send` and `Sync` implementations.

**2026-02-07 - [Fix 32-bit Integer Overflow in Security Checks]**
**Threat:** `width * height` calculation in security assertions could wrap on 32-bit systems, bypassing the buffer size check.
**Defense:** Replaced multiplication with `.checked_mul().expect(...)`.
