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
**2026-02-28 - TileRenderer Heap Overflow in parallel mode**\n**Threat:** The `TileRenderer::render_batch` function lacked bounds checking for `Framebuffer` and `ZBuffer` dimensions when using the `parallel` feature. Since it heavily relies on `SendPtr::write` for non-overlapping memory regions, passing smaller framebuffers led to a Heap Buffer Overflow due to writing out of bounds into memory.\n**Defense:** Added rigorous bounds checks at the entry point of `TileRenderer::render_batch` matching the expected widths and heights initialized in `TileRenderer::new`. Also updated `time` dependency to version `0.3.47` to fix DoS vulnerability.
**2026-03-01 - [Fix scanline rasterizer buffer overflows]**
**Threat:** Several rasterizer functions (`texture.rs`, `phong.rs`, `pbr.rs`, `reflection.rs`) calculated bounds manually and then directly used `unsafe { fb.get_unchecked_mut(...) }`. An off-by-one error or uncaught negative coordinate could lead to buffer overflows and memory corruption.
**Defense:** Refactored all affected scanline functions to use `crate::rasterizer::core::prepare_scanline`, which safely computes clamped slice boundaries and returns valid mutable slices of the framebuffer and Z-buffer, entirely removing the `unsafe` block for bounds extraction.
**2025-01-20 - Unbounded Allocation DoS in L-System Generator**
**Threat:** The `LSystem::expand()` function in `src/experimental/arboretum.rs` calculated lengths by unbounded scalar addition inside a loop based on the `iterations` count, then called `Vec::with_capacity(exact_len)`. An attacker could pass a large iteration count (or craft a specific rule) causing an exponential memory explosion that crashes the server or application with an Out-of-Memory (OOM) panic. Also contained an `unsafe { String::from_utf8_unchecked }` block.
**Defense:** Added a `100_000_000` (100MB) length limit and used safe `checked_add` math combined with early-return `Result` wrapping. Replaced the `unsafe` block with standard safe `String::from_utf8`.

**2025-01-20 - Buffer Overflow in SoftBody SIMD**
**Threat:** The `SoftBody::update_simd` physics loop in `src/experimental/jelly.rs` gathered data from internal arrays via AVX2 using indices loaded straight from memory into SIMD registers (`_mm256_i32gather_ps(pos_base, idx_a)`), where `idx_a` came from `spring_indices_a`. If a user mutated `spring_indices_a` post-creation (as it was fully `pub`), or provided out-of-bounds indices, the physics step would read/write wildly out of bounds, risking arbitrary memory corruption or info leaks.
**Defense:** Changed the struct fields to `pub(crate)` and added an explicit `O(N)` bounds validation check immediately prior to the SIMD loop for defense-in-depth, alongside an encapsulated `add_spring` method.

**2023-10-27 - [Out of Bounds / Uninitialized Memory Read in ClippedTriangles]**
**Threat:** The `Index` trait implementation for `ClippedTriangles` in `src/clipping.rs` used `debug_assert!` to verify that the requested index was within the valid initialized bounds (`self.count * 3`). In release builds, `debug_assert!` is stripped out, causing the bounds check to be entirely removed. This would allow an out-of-bounds index to call `unsafe { self.tris[index].assume_init_ref() }`, leading to either reading uninitialized memory (`MaybeUninit`) or a buffer overflow reading past the array's maximum capacity (24), which is Undefined Behavior and a potential crash or memory leak vector.
**Defense:** Replaced `debug_assert!` with a strict `assert!` in the `Index` trait implementation. This guarantees that bounds checking is always performed, regardless of the build profile, safely panicking the thread instead of triggering Undefined Behavior.
