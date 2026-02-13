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

**2025-05-29 - [Fix DoS via Unbounded Allocation]**
**Threat:** Denial of Service via memory exhaustion/panic. `Framebuffer::new`, `ZBuffer::new`, and `Texture::new` allowed allocating buffers up to 16GB (u32::MAX pixels), causing panic or OOM kill.
**Defense:** Enforced a maximum dimension of 16,384 pixels (268 MP) in constructors. Added `tests/security_limits.rs` to verify safe failure.
