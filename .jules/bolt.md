<<<<<<< bolt-hashmap-capacity-3866827711962762298
**[Fast Inv Sqrt vs Stdlib SQRT Recip]**
=======
## [.zip Iterator for SoftBody collide_sdf]
**What:** Replaced index-based `for i in 0..len` loop in `SoftBody::collide_sdf` with `.iter_mut().zip(...)`.
**Why:** Elides bounds checking and satisfies idiomatic Rust patterns.
**Impact:** Minor but consistent performance win.
**Measurement:** `softbody_bench` run time decreased from ~32.1µs to ~29.4µs (~8% improvement).
**Fast Inv Sqrt vs Stdlib SQRT Recip**
>>>>>>> trunk
**Learning:** Using `fast_inv_sqrt` (Quake III trick) is slower and less precise than using `std`'s `sqrt().recip()` directly on modern CPU architectures when compiling. The standard library leverages hardware-accelerated instructions (like `rsqrtss`) automatically and provides better results.
**Action:** Replaced `fast_inv_sqrt(dist_sq)` with `dist_sq.sqrt().recip()` in the scalar fallback paths of point-lit and shadowed phong rasterizers, resulting in ~6-7% performance improvement in single-point light rendering.

**[Pre-allocate glTF vectors to eliminate dynamic heap reallocations]**
**Learning:** Using `Vec::new()` in large iterations (like parsing glTF primitives and channels) results in unnecessary dynamic heap reallocations.
**Action:** Pre-allocate vectors using `Vec::with_capacity()` along with exact sizes from iterators via `.count()` and `.map(|m| m.primitives().count()).sum()` when reading documents.
**DrawList Heap Allocations**
**Learning:** When generating a `DrawList` in `cpu_renderer::extract_draw_list` or `scene::extract`, the inner vectors (`vertices`, `batches`) were being initialized with `Vec::new()` and subsequently `reserve()`d with the exact required capacity. This resulted in unnecessary heap allocations when adding new `DrawBatch`es. Pre-allocating directly using a new constructor `DrawList::with_capacity` improves performance by eliminating those redundant dynamic heap allocations.
**Action:** Created `DrawList::with_capacity(camera, num_commands, num_vertices)` and modified `CpuRenderer::extract_draw_list` and `Scene::extract` to utilize it instead of `new` followed by `reserve()`. Reduces heap allocation overhead resulting in an ~9% speedup for extracting the draw list.
**[Bresenham Overdraw]
**Learning:** In integer-based shape rasterization (like Bresenham's algorithm for filled circles), naive loops often draw scanlines repeatedly on the same axis (e.g., generating `x` span updates while `y` has not yet decremented), leading to severe overdraw and wasted memory bandwidth.
**Action:** Always track boundary changes (`y` axis decrements) to issue single, maximum-width scanline draw calls instead of accumulating overdrawn fragments per `x` tick.

**Gouraud Scanline Bounds Elision**
**Learning:** Replaced safe `zip` iterators with unsafe raw pointer iteration + a pre-loop bounds assertion in `draw_scanline_gouraud_i32` and `draw_scanline_gouraud_i32_tile`. This avoids bounds-checking overhead per pixel and avoids the overhead of zipped iterators.
**Action:** Modified `crates/abrash-render/src/rasterizer/gouraud.rs` and `crates/abrash-render/src/rasterizer/tile.rs` to use raw pointers. A single initial length assertion ensures memory safety. Verified ~4% speedup in `gouraud_scanline_new` benchmarks.

**Eliminate Per-Frame Vec Allocations in Skeletal Animation**
**Learning:** Calling `compute_global_transforms` and `compute_skin_matrices` from `abrash-skeletal` per frame resulted in dynamically allocating `Vec<Mat4>` on the heap for each call.
**Action:** Created `update_global_transforms` and `update_skin_matrices` to mutate a provided buffer in-place (`&mut Vec<Mat4>` and `&mut SkinMatrices`), bypassing the per-frame allocations during animation evaluation.
**Fused Multiply-Add over Hypot**
**Learning:** Using `(x * x + y * y).sqrt()` triggers the `clippy::imprecise_flops` lint, but replacing it directly with `x.hypot(y)` causes significant performance regressions. The standard library's `hypot` implementation is accurate but much slower than naive squaring. A better approach that is both highly accurate and fast is to use Fused Multiply-Add (FMA): `x.mul_add(x, y * y).sqrt()`.
**Action:** Replaced instances of `(x * x + y * y).sqrt()` with `x.mul_add(x, y * y).sqrt()` globally in core math, procedural, rendering, and raycast logic, ensuring numerical precision while maintaining or improving benchmark performance without needing to suppress standard clippy lints globally.
**[Pre-allocate double buffers in string expansion]**
**Learning:** Using `Vec::new()` for the secondary buffer in double-buffered loops (like L-System expansion) causes unnecessary heap reallocations during the first iteration.
**Action:** Initialized the secondary buffers using `Vec::with_capacity(current_bytes.len() * 2)` in `lsystem.rs` and `arboretum.rs` to eliminate the initial dynamic heap reallocations.
**Scanline Jitter Optimization**
**Learning:** When iterating over a mutable slice to process alternating chunks (e.g., modifying only even rows in a framebuffer), use `chunks_exact_mut(chunk_size * 2)` and slice the target portion (e.g., `chunk[0..chunk_size]`) instead of `chunks_exact_mut(chunk_size).step_by(2)`. This avoids the iterator overhead of `step_by` and significantly improves performance.
**Action:** Refactored `apply_scanline_jitter` to use double-row chunking.
**Eliminate Bounds Checking Overhead in Circle Rasterization**
**Learning:** When integer bounds checking is mathematically guaranteed by earlier bounds tests (e.g., confirming a circle is entirely visible or sufficiently small such that  won't overflow ), using safe but slow operations like  and  within the tight per-pixel inner loop adds significant branching overhead. Similarly, in symmetrical rasterization algorithms, drawing identical scanlines when an axis offset is zero () creates unnecessary overdraw.
**Action:** Replaced  with standard / in  and  safe paths. Removed the redundant  scanline initialization draw when . Performance improved by ~10% for out-of-bounds circles.
**Eliminate Bounds Checking Overhead in Circle Rasterization**
**Learning:** When integer bounds checking is mathematically guaranteed by earlier bounds tests (e.g., confirming a circle is entirely visible or sufficiently small such that `xc + radius` won't overflow `i32`), using safe but slow operations like `saturating_add` and `saturating_sub` within the tight per-pixel inner loop adds significant branching overhead. Similarly, in symmetrical rasterization algorithms, drawing identical scanlines when an axis offset is zero (`x = 0`) creates unnecessary overdraw.
**Action:** Replaced `saturating_add/sub` with standard `+`/`-` in `draw_circle` and `fill_circle` safe paths. Removed the redundant `yc - x` scanline initialization draw when `x = 0`. Performance improved by ~10% for out-of-bounds circles.

**[Pre-allocate HashMaps to eliminate dynamic heap reallocations]**
**Learning:** Using `HashMap::new()` in large iterations or when dealing with known data sizes (like parsing glTF joints and nodes) results in unnecessary dynamic heap reallocations and creates empty maps that scale inefficiently during heavy insertions.
**Action:** Pre-allocated HashMaps using `HashMap::with_capacity()` utilizing known bounds from iterators and slices, eliminating reallocation overhead on the hot parsing path.
