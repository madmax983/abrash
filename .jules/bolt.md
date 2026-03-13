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

**[Performance Optimization: Inline multi-element struct collection instead of .collect() mapping]**
**Learning:** When building an output collection from an algorithm that produces elements in sequence (like Marching Cubes/Tetrahedra producing 3 indices at a time), directly pushing grouped elements (e.g. `[usize; 3]`) avoids an O(N) intermediate heap allocation and mapping pass that would be caused by a `.collect()` chain on `.chunks(3)`.
**Action:** When a function requires a specific grouped structural return (like `Vec<[usize; 3]>`), maintain this structure inline as elements are computed rather than collecting into a flat list and refactoring it at the very end.
**[Performance Optimization: Cross-multiplication for variance comparisons]**
**Learning:** In color filtering algorithms (like Kuwahara), replacing floating-point division and casts for variance comparisons (`var1 < var2` where `var = num / den`) with integer cross-multiplication (`num1 * den2 < num2 * den1`) completely removes `f32` casts and operations from the hot loops.
**Action:** Always prefer cross-multiplication with integer types (e.g. `u64`) over floating point division when comparing ratios or variances in tight pixel processing loops.
**[Performance Optimization: Thread-local buffers for coordinate remapping]**
**Learning:** When reusing `thread_local!` buffers for screen-space post-processing effects that remap coordinates (like CRT barrel distortion), explicitly call `.fill(default_color)` on the reused buffer. This prevents visual 'ghosting' artifacts from previous frames in areas (like screen corners) that aren't overwritten by the distorted image, while maintaining the zero-cost allocation benefits.
**Action:** Always explicitly `.fill()` thread-local slice borrows if the rendering algorithm doesn't guarantee writing to every single pixel in the destination slice.

## [Radial Blur Optimization]
**Concept:** Optimizing the Radial Blur post-processing effect by eliminating float-to-int conversions inside the innermost rendering loop.
**Fate:** Merged
**Lesson:** Fixed-point math and digital differential analyzer (DDA) techniques provide massive speedups when doing sub-pixel interpolation. By pre-calculating the step interval in fixed point (`let step_x = (dx * step_factor * 65536.0) as i32`), replacing the sample array allocation and float math (`for &scale in scales { ... dx * scale }`) with integer accumulation (`cur_x += step_x`), the benchmark execution time was effectively halved (~45% speedup).

## Optimize transform_batch with collect

**Learning:** Replacing a manual `for` loop and `.push()` with an idiomatic `.map(...).collect()` chain on a slice iterator can yield significant performance gains. Because slice iterators implement the `TrustedLen` trait, `.collect()` can safely bypass bounds and capacity checks on every insertion, allowing LLVM to better vectorize the code.
**Action:** Always prefer iterator chains over manual loop pushes when transforming slices or arrays, as it not only improves readability but can also drastically enhance performance by leveraging zero-cost abstractions.
**[Performance Optimization: Look-Up Tables for Color Channels]**
**Learning:** When performing per-pixel math operations on 8-bit color channels (like brightness and contrast adjustments), there are only 256 possible input values. Re-calculating the math and clamping bounds for millions of pixels per frame is redundant.
**Action:** Replace per-pixel inner-loop calculations with a precomputed 256-element Look-Up Table (LUT) (`[u32; 256]`). This converts complex math and clamping logic into a simple `O(1)` array indexing operation per channel, providing massive speedups on large framebuffers.
**[Performance Optimization: Integer Fixed-Point Lerping]**
**Learning:** In pixel blending or interpolation hot loops, replace floating-point `lerp` operations with integer fixed-point arithmetic. For example, scaling a `0.0-1.0` blend factor to a `0-256` integer, then computing `(a * inv_factor + b * factor) >> 8`.
**Action:** Always prefer integer fixed-point math over floating-point linear interpolation for per-pixel color blending to significantly improve rendering performance.

## Raytracer Parallel Allocation Elimination
**Learning:** `scene.objects.iter().map(|obj| RenderObject { obj, world_aabb: obj.calculate_world_aabb() }).collect::<Vec<_>>()` created unnecessary allocations within a parallel rendering loop.
**Action:** Replaced `collect::<Vec<_>>()` with a `thread_local!` `RefCell<Vec<AABB>>` buffer to store computed bounds. Using Structure-of-Arrays (`scene.objects` and `AABB_BUFFER`), we eliminated dynamic allocations inside the render loop, which improved execution times from ~82ms to ~21ms (74% improvement).
**[Performance Optimization: Zip Iterator to Eliminate Array Clones]**
**Learning:** In procedural mesh modifiers (like noise displacement), unnecessary O(N) heap allocations can be avoided by making sure the normal array is correctly sized in place, then using `.zip(&mesh.normals)` next to the `mesh.vertices.par_iter_mut()` loop to avoid cloning the normal array.
**Action:** Always favor `.zip()` and in-place resizing instead of `.clone()` for concurrent or parallel array iterations.
**[Performance Optimization: Fixed-Point Directional Blur]**
**Learning:** In sub-pixel sampling loops like directional blurs, replacing floating-point coordinate math and `.round()` operations with 16.16 fixed-point integer arithmetic provides an enormous ~75% speedup by eliminating `f32` conversion overhead in the inner-most rendering loop. Scaling steps by `65536.0` (`<< 16`) and offsetting the initial starting coordinates by `32768` (0.5 in 16.16 fixed point) achieves free mathematical rounding via standard integer truncation when extracting the coordinate (`>> 16`).
**Action:** Always prefer integer fixed-point math over floating-point arithmetic for coordinate sampling and interpolation in per-pixel rendering loops to significantly improve performance.
**[Performance Optimization: Fixed-Point Coordinates for Sub-pixel Sampling]**
**Learning:** In sub-pixel sampling loops (like directional blurs), calculating coordinates using floating point math, adding steps, and calling `.round()` per-sample is a massive bottleneck.
**Action:** Replace floating-point coordinate math with 16.16 fixed-point integer arithmetic. Scale steps by `65536.0` (as `i32`) outside the loop. Offset the initial starting coordinates by `32768` (representing 0.5 in 16.16 fixed-point) and accumulate integers in the loop. The final integer pixel coordinate can then be extracted simply via a right shift (`>> 16`), which naturally incorporates the 0.5 rounding cost-free.
**[Performance Optimization: Zip Iterator to Eliminate Array Clones]**
**Learning:** In procedural mesh algorithms like `displace_noise`, cloning entire vectors (e.g. `mesh.normals.clone()`) to satisfy borrow checker rules or avoid length mismatches introduces an unnecessary `O(N)` heap allocation. Furthermore, using `.enumerate()` to access a separate slice via `let normal = normals[i];` incurs implicit bounds-checking.
**Action:** Replace `vector.clone()` with explicit length matching and `.resize()` in place. Then, use `.zip(&mesh.normals)` when iterating over `mesh.vertices.par_iter_mut()` to safely obtain simultaneous mutable and immutable borrows, entirely eliminating both the heap allocation and the inner-loop bounds checks.
## [Performance] Zero-Cost Normal Iteration in Displace Noise
**Learning:** In procedural mesh modifiers, cloning `mesh.normals` just to iterate alongside `mesh.vertices` creates an unnecessary $O(N)$ heap allocation per frame/call.
**Action:** Exploit Rust's ability to take disjoint borrows from the same struct by properly sizing `mesh.normals` in-place, and directly pairing it with `mesh.vertices` using `.zip(&mesh.normals)`. This removes the clone while preserving `.par_iter_mut()` parallelism entirely overhead-free.

**[Performance Optimization: Flattened iterator chunking for framebuffer exports]**
**Learning:** When flattening or transforming 1D slices representing 2D grids (like framebuffers), using manual `y` and `x` loops with `.push()` operations incurs overhead from bounds checking on every insertion. Furthermore, if `pixels.len()` is larger than `width * height` (e.g., due to memory padding), iterating over `chunks_exact(width)` without `.take(height)` can incorrectly write out-of-bounds rows.
**Action:** Use `.chunks_exact(width).take(height)` combined with `.extend(row.iter().flat_map(...))` to bypass repetitive bounds checks, enable loop vectorization, and correctly handle capacity alignment padding without panicking or writing garbage data.

## Bolt's Journal
**[Eliminate Bounds Checks in 2D Iteration]**
**Learning:** In hot paths iterating over an entire framebuffer or 2D grid, nested `x`/`y` loops that use bounds-checked `get_pixel(x, y)` calls introduce significant overhead due to the bounds check and `Option` unwrapping per pixel.
**Action:** Replace nested `x`/`y` loops with direct slice iteration using `.as_slice().chunks_exact(width)`. This leverages zero-cost abstractions to elide bounds checks and eliminate `Option` unwrapping overhead, enabling better vectorization and significant performance gains.

## Double-Buffering L-System Expansions
**Learning:** In string or byte expansion loops where a new string is generated from an old one (like Lindenmayer Systems), allocating a new `String` or `Vec` on every iteration (`let mut next = String::with_capacity(len);`) causes significant memory allocation and deallocation churn in a hot path.
**Action:** Use double-buffering. Declare both the `current` and `next` buffers outside the expansion loop. Inside the loop, call `.clear()` and `.reserve(exact_len)` on the `next` buffer, and then use `std::mem::swap(&mut current, &mut next)` at the end of the iteration. This strictly limits heap allocations to only when the string outgrows its previous capacity.
