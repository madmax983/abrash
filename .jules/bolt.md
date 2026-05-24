[Output truncated for brevity]

**[Eliding huge capacity overallocations]**
**Learning:** In exponential string generation algorithms (like L-Systems), unconditionally pre-allocating strings with an extremely large OOM-prevention limit (e.g., `String::with_capacity(max_capacity)`) causes severe initialization overhead.
**Action:** Use `String::new()` and rely on natural allocator capacity growth. This is significantly faster and prevents massive heap over-allocation for simple configurations.

**[Heat Vision LUT Optimization]**
**Learning:** In the `apply_heat_vision` effect, replacing the dynamic conditional branches `if t < 256 { ... }` within the per-pixel hot loop with a pre-calculated 1024-entry lookup table (LUT) eliminates branching and arithmetic overhead entirely for color resolution mapping.
**Action:** Replaced conditional arithmetic and shift-operations with a `[u32; 1024]` lookup table, leading to a massive 67% performance increase across all resolutions (e.g. 800x600 improved from ~8.8ms to ~2.9ms).

**[Array Destructuring Extend]**
**Learning:** Replacing sequential `.push()` calls within a hot loop with a single `.extend([a, b, c])` call using array destructuring significantly improves performance by allowing the compiler to elide repetitive vector bounds checks.
**Action:** When adding multiple items to a `Vec` in a tight loop, prefer `extend` with a fixed-size array over sequential `push` calls.

**[String formatting in hot loops is extremely slow]**
**Learning:** String formatting via the `write!` macro in hot per-pixel loops (e.g., generating ANSI sequences) incurs severe overhead due to dynamic format parsing and trait dispatch.
**Action:** Replace `write!` with a custom, allocation-free `itoa`-style integer formatter using a small byte buffer and remainder math to drastically improve performance (e.g., ~85% reduction in execution time for ASCII string generation). Also, remember to place helper functions at the top of the block to avoid `clippy::items_after_statements` lint errors.

**Optimize SSAO Kernel Loop**
**Learning:** In hot loops, replacing `kernel[k]` with a dereferenced iterator value (`&s`) avoids redundant array indexing and bounds checking.
**Action:** Replaced `let s = kernel[k];` with `for (k, &s) in kernel.iter().enumerate().take(KERNEL_SIZE)` in `crates/abrash-render/src/post_process/ssao.rs`.

## [Compile-Time LUT Generation]
**Bottleneck:** Pre-calculating a gradient color Look-Up Table (LUT) dynamically inside a hot post-processing function (`apply_heat_vision`) incurs initialization overhead on every frame.
**Optimization:** Extracted the generation loop into a `const fn` (replacing `for` loops with `while` loops due to `const fn` restrictions) and declared `const LUT: [u32; 1024] = generate_lut();` inside the function.
**Impact:** Eliminated dynamic array allocation and setup overhead per frame, maximizing throughput in headless render loops.

**[Eliminating Redundant Buffer Allocations in Full-Screen Overwrites]**
**Learning:** When a post-processing effect completely overwrites the framebuffer (e.g., drawing a procedural tunnel) without reading the previous frame state, allocating a temporary buffer (`vec![0; width * height]`) is an unnecessary O(W*H) heap allocation per frame.
**Action:** Directly mutate the framebuffer slice (e.g., via `framebuffer.as_mut_slice().chunks_exact_mut(...)`). This entirely eliminates the need for an intermediate buffer and avoids the O(N) secondary loop required to copy pixels back to the screen.

**[Loop Fusion Optimization]**
**Learning:** In hot rendering paths, sequentially iterating over the same collection multiple times (e.g., first to validate and count totals, second to calculate subset bounds or ranges) introduces redundant memory accesses, redundant resource lookups (like mesh fetching), and bounds checking.
**Action:** Fuse sequential iteration passes over the same collection into a single pass when the calculations are mathematically independent but contextually aligned.

**[Extracting Constant Sub-expressions in 2D Loops]**
**Learning:** In 2D per-pixel processing loops (e.g., computing a vignette effect), recalculating mathematically independent sub-expressions that only vary by one axis (like `dx_sq_scaled = dx * dx * scale`) on every iteration inside the inner horizontal loop forces redundant floating-point math per pixel.
**Action:** Hoist the constant 1D expressions into a precomputed array outside the vertical loop (`let mut dx_sq_scaled_cache = vec![0.0; width]`), and pair the cached values with the inner row iterator using `.zip()`. This eliminates repetitive floating-point math per pixel, significantly boosting performance (e.g., ~50% execution time reduction).

**[Extracting Constant Sub-expressions in 2D Loops]**
**Learning:** In 2D per-pixel processing loops (e.g., computing a vignette effect), recalculating mathematically independent sub-expressions that only vary by one axis (like `dx_sq_scaled = dx * dx * scale`) on every iteration inside the inner horizontal loop forces redundant floating-point math per pixel.
**Action:** Hoist the constant 1D expressions into a precomputed array outside the vertical loop (`let mut dx_sq_scaled_cache = vec![0.0; width]`), and pair the cached values with the inner row iterator using `.zip()`. This eliminates repetitive floating-point math per pixel, significantly boosting performance (e.g., ~50% execution time reduction).

**[Extracting Constant Sub-expressions in 2D Loops]**
**Learning:** In 2D per-pixel processing loops (e.g., computing a vignette effect), recalculating mathematically independent sub-expressions that only vary by one axis (like `dx_sq_scaled = dx * dx * scale`) on every iteration inside the inner horizontal loop forces redundant floating-point math per pixel.
**Action:** Hoist the constant 1D expressions into a precomputed array outside the vertical loop (`let mut dx_sq_scaled_cache = vec![0.0; width]`), and pair the cached values with the inner row iterator using `.zip()`. This eliminates repetitive floating-point math per pixel, significantly boosting performance (e.g., ~50% execution time reduction).
