# Bolt's Journal

**[Performance Optimization: Unchecked Rasterizer Access]**
**Learning:** Removing bounds checks (`test_and_set` -> `test_and_set_unchecked`) in the single-pixel/empty-span path of the rasterizer yielded a ~20% performance improvement for small triangles. This path is hit frequently for sub-pixel or thin geometry.
**Action:** Look for other "checked by logic" hot paths where `unsafe` unchecked access can be justified by loop invariants.

**[Optimization Trap: Intrinsics vs Inlining]**
**Learning:** Attempting to optimize `Vec3::normalize` with `unsafe` SIMD intrinsics (`rsqrtss`) caused precision regressions in tests and violated safety policies. However, simply adding `#[inline]` to `Vec3::length` provided a ~50% speedup (11.3ns -> 5.7ns), matching the performance of the unsafe intrinsic version without the downsides.
**Action:** Always profile function call overhead and inlining (`#[inline]`) before resorting to `unsafe` intrinsics. Precision loss from fast-math approximations can break regression tests.

**[Performance Optimization: Rayon fold to flat_map_iter]**
**Learning:** Replacing `.fold(Vec::new, ...).flatten().collect()` with `.flat_map_iter(...)` in Rayon iterator chains eliminates intermediate `Vec` allocations per chunk. This reduces heap allocations significantly in parallel processing pipelines like the tile rasterizer (`submit_mesh`, `render_batch` paths).
**Action:** Always prefer `flat_map_iter` when flattening iterator results in Rayon to keep data on the stack (via returned Iterators or arrays) and avoid heap-allocating intermediate collections.

**[Performance Optimization: Idiomatic Iteration in Hot Loops]**
**Learning:** Replacing a manual index-based iteration loop (`for x in 0..width`) with an idiomatic iterator `row.iter_mut().enumerate().take(width)` inside the inner `for_each` of SSAO implementation improved the overall `apply_ssao` benchmark by 5-15%. This occurs because the iterator approach allows the LLVM compiler to elide the implicit array bounds checking that index access necessitates, especially inside complex nested loop structures involving nested pixel modifications.
**Action:** When performing pixel-by-pixel operations inside a 1D slice (or a sub-slice like a row), always prefer `slice.iter_mut().enumerate()` over `for x in 0..len { slice[x] }` to remove bounds-checking overhead.
**Optimize capacity estimations for large procedurally generated datasets**
**Learning:** While `Vec::with_capacity()` is standard practice to avoid allocations, adding an O(N) pre-pass to count specific elements (like `[` in an L-System string) or calculate exact replacement lengths for unicode characters to calculate an exact capacity for extremely long strings or datasets can cause a massive performance regression.
**Action:** When working with large procedurally expanded data, use a reasonable O(1) heuristic capacity estimate based on the current length (e.g. `String::with_capacity(current.len() * 2)`) and let the collection scale dynamically, avoiding full iteration overhead.

**[Performance Optimization: Zip window iterators]**
**Learning:** Replacing manual index-based bounds checking in convolution filters (like `apply_emboss`) with `.windows(3)` iterators zipped with a mutable destination row slice completely elides bounds checking in the inner loop, yielding a ~2-3% performance improvement on large framebuffers.
**Action:** When working with 1D slices conceptually representing 2D grids (like image processing convolutions), always prefer zipping `.windows(kernel_size)` over manual indexing (`row[x - 1]`, `row[x + 1]`) to give the compiler maximum optimization opportunities.
**[Performance Optimization: Rayon par_extend to remove intermediate Vec collections]**\n**Learning:** When using Rayon `flat_map_iter`, using `.collect::<Vec<_>>()` followed by `.extend()` into an existing vector forces a per-frame heap allocation. Replacing this with `target_vec.par_extend(iterator)` allows Rayon to insert directly into the target collection in parallel, entirely avoiding the intermediate `Vec` heap allocation in hot paths like `TileRenderer::render_batch`.\n**Action:** Use `.par_extend()` on existing collections when accumulating results from Rayon parallel iterators to avoid creating intermediate collections.\n
Persona 'Bolt' Learning: In convolution/blur algorithms, replace per-pixel floating-point color accumulations and i32 conversions with integer accumulators using unsigned math and saturating operations (e.g. `(pos + bias).saturating_sub(neg).min(255)`) to avoid cast overheads and keep inner loops purely integer-based.

**[Performance Optimization: Thread-local buffers in parallel iteration]**
**Learning:** Using `.for_each_init(|| vec![0u32; len], ...)` or allocating a `Vec` per element inside a Rayon parallel iterator (`par_iter`) causes a high number of dynamic heap allocations per frame, particularly when iterating over elements like pixels or columns.
**Action:** Replace `for_each_init` allocations inside parallel loops with a `std::thread_local! { static BUFFER: std::cell::RefCell<Vec<T>> = ... }` buffer. This guarantees only one allocation occurs per Rayon worker thread, allowing safe reuse of capacities across elements by resizing as needed and borrowing mutable slices.

**[Performance Optimization: Cross-multiplication for variance comparisons]**
**Learning:** In color filtering algorithms (like Kuwahara), replacing floating-point division and casts for variance comparisons (`var1 < var2` where `var = num / den`) with integer cross-multiplication (`num1 * den2 < num2 * den1`) completely removes `f32` casts and operations from the hot loops.
**Action:** Always prefer cross-multiplication with integer types (e.g. `u64`) over floating point division when comparing ratios or variances in tight pixel processing loops.
