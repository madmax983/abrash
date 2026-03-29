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
**2025-05-15 - [Fix potential Out-of-Bounds in Kuwahara filter]**
**Threat:** The `apply_kuwahara` function in `src/experimental/kuwahara.rs` used `unsafe { *src_fb.get_unchecked(...) }` when sampling pixels. Even though boundaries were explicitly clamped to logical limits, casting `width` to `i32` and complex row offsets calculated by hand introduced a risk of memory corruption.
**Defense:** Replaced the `unsafe` block with safe slice indexing, ensuring bounds checks remain, deferring any optimization safely to the compiler.

**2026-03-05 - [Fix Integer Overflow in Texture Fast Path]**
**Threat:** Integer overflow in the fast path condition of `draw_span_nearest` and `draw_span_bilinear` in `src/rasterizer/texture.rs`. The code checked if the bounds calculated with `i64` were valid, but the inner loop used `i32` arithmetic. If `du_fix` or `dv_fix` were extremely large, the `i32` variables would wrap around, leading to massively out-of-bounds `usize` indices and unsafe out-of-bounds memory accesses.
**Defense:** Added `u_max_64 <= i32::MAX as i64` and `v_max_64 <= i32::MAX as i64` to the `can_use_fast_path` conditions, ensuring that no `i32` wrap-around can occur during the loop execution.

**2025-03-16 - [TileRenderer Parallel Buffer Overflow]
**Threat:** `TileRenderer` parallel worker threads write to shared `Framebuffer` and `ZBuffer` using `unsafe` raw pointers (`SendPtr`). The buffer dimensions were assumed identical to the renderer's, but logic-only `assert_eq!` bounds checks on `width` and `height` didn't verify the actual slice lengths, allowing a tiny buffer (e.g. `1x1`) to be overwritten massively by a large configured tile renderer (Heap Buffer Overflow).
**Defense:** Explicitly assert that `fb.as_slice().len()` and `zb.as_slice().len()` are `>=` to the TileRenderer's `width.checked_mul(height).expect("...")` *before* slicing the pointers and passing them into parallel threads. Using `checked_mul` is strictly required to prevent integer overflow bypasses where a malicious width and height wrap around to match a tiny buffer. Using `>=` prevents strict equality failures when there's internal buffer padding.
**2025-03-21 - [TileRenderer Out-of-Bounds Write]**
**Threat:** An `unsafe` block in `SendPtr::write` (used by `TileRenderer` for parallel rendering) was performing unverified raw pointer writes to the framebuffer and zbuffer. A user could spoof the expected dimensions of the TileRenderer or rely on integer overflow logic in `checked_mul` bounds checks to bypass the slice `.len() >= expected_len` checks on 64-bit systems. This allowed a small, artificially constrained framebuffer slice to be written to out of bounds when processing triangles via `SendPtr::write`.
**Defense:** Modified `SendPtr<T>` to contain the maximum valid length of the underlying slice (`struct SendPtr<T>(*mut T, usize);`). Added a strict runtime assertion `assert!(index < self.1, "Index out of bounds")` directly inside the `SendPtr::write` method before the unsafe pointer offset operations.
**2024-03-26 - [Fix Generation Overflow in ResourcePool]**
**Threat:** A generational resource pool implementation using `u32` for its generation counter incremented the generation on every removal, and placed the freed slot back into the free list. A long-running server or a malicious actor could cycle a slot through `u32::MAX` creations/deletions, wrapping the generation counter back to 0. This could either cause a panic (DoS) on debug builds due to overflow checking, or lead to a Use-After-Free vulnerability on release builds, allowing an attacker to access newly inserted unrelated data using an old handle with the wrapped generation.
**Defense:** Explicitly prevent the generation counter from wrapping. If `*generation == Generation::MAX` upon removal, the slot is marked as `Vacant { generation: Generation::MAX }` but its index is **not** pushed to the `free_list`. The slot becomes permanently exhausted, gracefully mitigating both the DoS panic and the Use-After-Free without requiring a larger, slower generation type.
**2024-05-15 - Integer overflow in circle rasterization**
**Threat:** A negative radius can bypass bounds checking, causing out-of-bounds geometry to falsely pass the check and trigger Undefined Behavior (slice out-of-bounds panic).
**Defense:** Added an explicit `if radius < 0 { return; }` validation before bounding box calculations to ensure only valid geometries are processed.
