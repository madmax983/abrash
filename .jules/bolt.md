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
**[Performance Optimization: Rayon par_extend to remove intermediate Vec collections]**
**Learning:** When using Rayon `flat_map_iter`, using `.collect::<Vec<_>>()` followed by `.extend()` into an existing vector forces a per-frame heap allocation. Replacing this with `target_vec.par_extend(iterator)` allows Rayon to insert directly into the target collection in parallel, entirely avoiding the intermediate `Vec` heap allocation in hot paths like `TileRenderer::render_batch`.
**Action:** Use `.par_extend()` on existing collections when accumulating results from Rayon parallel iterators to avoid creating intermediate collections.

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
**[Performance Optimization: Par Chunks Exact Mut in Film Grain]**
**Learning:** Using Rayon's `par_iter_mut().enumerate()` across millions of individual pixels creates high task-spawning and synchronization overhead.
**Action:** Replace it with `.par_chunks_exact_mut(width).enumerate()` to parallelize per-row instead of per-pixel. Calculate the true pixel index by combining the row offset and the chunk index. In the `apply_film_grain` filter, this improved the benchmark time from ~10.7ms down to ~1.8ms per 1080p frame (nearly 6x speedup) while still cleanly supporting the pixel-index dependent pseudorandom noise generation without allocating buffers.
**[Performance Optimization: Eliminate Manual Slice Bounds Checks in 2D Block Iteration]**
**Learning:** In operations that write to a sub-region (a rectangle) of a 1D slice representing a 2D grid, manually calculating array boundaries inside a for loop `for row in start_y..end_y` via `row * width + start_x` triggers implicit bounds checking on every single row assignment.
**Action:** Replace `for row in start_y..end_y` with `.chunks_exact_mut(width).take(end_y).skip(start_y)`. This yields direct access to the exact sub-slice `row[start_x..end_x]` and completely elides runtime array bounds checking on the main array inside the hot loop, significantly improving performance (e.g. ~50% speedup for clearing 4k framebuffers).
**[Performance Optimization: Pre-computed ASCII Array Lookup for L-System Expansion]**
**Learning:** In string rewriting systems (like L-Systems) that primarily use ASCII characters, iterating over `.chars()` and looking up replacements in a `HashMap<char, String>` incurs significant overhead from UTF-8 decoding, bounds checking, and `SipHash` calculations on every single character.
**Action:** Pre-compute a fixed-size array lookup (`[Option<&str>; 128]`) for all ASCII replacement rules. Iterate over the input string using `.chars()` and use the array index for O(1) lookups if `c as u32 < 128`. Fall back to the `HashMap` only for non-ASCII characters. Additionally, hoist OOM capacity checks (`if next_string.len() > limit`) outside the inner loop. In L-System expansion, this yielded a ~75% performance improvement.
\n**[Performance Optimization: Eliminate Manual Slice Bounds Checks in Hot 2D Block Iteration]**\n**Learning:** In operations that iterate over a 2D grid, manually calculating array boundaries inside a for loop `for y in 0..height` and `for x in 0..width` via `y * width + x` triggers implicit bounds checking on every single row assignment.\n**Action:** Replace nested index-based loops with `.chunks_exact_mut(width).enumerate().take(height)` and `.iter_mut().enumerate()`. This yields direct access to the exact element and completely elides runtime array bounds checking on the main array inside the hot loop, significantly improving performance.

**[Performance Optimization: Small Copy Types Pass-By-Value]**
**Learning:** Having method parameters or the `self` receiver passed by reference (`&self`) for small `Copy` structs (like a 3-float `Vec3`) causes unnecessary pointer indirection overhead during execution and can interfere with optimal register allocation in loops.
**Action:** Always prefer pass-by-value (`self`) instead of pass-by-reference (`&self`) for fundamental, small `Copy` math types to improve execution speed.

**[Performance Optimization: Safe SWAR integer overflow]**
**Learning:** When implementing SWAR (SIMD Within A Register) multiplication on packed color channels (e.g., Red and Blue masked within a `u32`), explicitly promote the masked values to `u64` before multiplication if the scale factor can cause the intermediate result to exceed `u32::MAX`. Failing to do so causes critical integer overflow bugs during pixel blending.
**Action:** Promote scaled SWAR values to `u64` where intensity/scale factor causes intermediate value representation constraints.

**[Performance Optimization: Eliminate per-frame memory allocation for Emboss]**
**Learning:** Calling `.to_vec()` on the framebuffer's slice inside `apply_emboss` causes an expensive memory allocation every frame, which hurts rendering performance.
**Action:** Use a `thread_local!` static buffer with `RefCell<Vec<u32>>`. Resize it to the required length and use `.copy_from_slice()` instead of `.to_vec()`. This effectively reuses the same allocation across all frames, functioning as a zero-cost double buffer.
**[Performance Optimization: Zip Iterator to elide array bounds checks in multi-slice iterations]**
**Learning:** To elide bounds checks when simultaneously iterating over multiple slices (e.g., reading a depth buffer and modifying a color buffer), replace index-based loops (`for i in 0..len`) with pre-sliced zipped iterators (`a[..len].iter_mut().zip(&b[..len])`). This idiomatic pattern allows LLVM to remove bounds checking in hot loops for measurable performance gains.
**Action:** Use zipped iterators bounded by exactly matching pre-slices for pixel post-processing logic that accesses `original_pixels`, `zb_slice`, or `blurred_slice` using index `[i]`.
**[Performance Optimization: Eliminate Panic dropping RefMut across parallel bounds]**
**Learning:** Using `thread_local!` with `RefCell<Vec<T>>` to eliminate per-frame allocations in functions that also use Rayon for parallelism (e.g., `apply_emboss`), do not hold the `RefMut` guard (`.borrow_mut()`) across parallel boundaries. This causes critical work-stealing panics. Additionally, dropping the value returned by `take()` using `.set()` throws compilation errors because `set` is a method on `Cell`, not `RefCell`. Use `.replace()` or mutate `.borrow_mut()` safely.
**Action:** Use `.replace()` on a `RefCell` or mutate `.borrow_mut()` when putting a taken value back inside `thread_local!` variables in Rayon-enabled loops.

**[Performance Optimization: Deferring Square Roots in Boids Simulation]**
**Learning:** In hot spatial simulation loops (e.g., Boids or flocking algorithms), distance calculations using multiple `sqrt` operations like `dx.hypot(dy).hypot(dz)` introduce massive overhead when applied across all $N^2$ entity pairs.
**Action:** Replace `hypot` calculations with squared distance comparisons (`dx * dx + dy * dy + dz * dz < radius_sq`). Calculate the actual `sqrt` only inside the conditional block, and only for the fraction of entities that are within range and explicitly require true distance values for weighting calculations (like separation).
## [Performance] ZBuffer clear_rect Optimization\n**Learning:** Clearing a sub-region of a 2D buffer (like a ZBuffer) using nested `x`/`y` loops with bounds checking per pixel is extremely slow. Also, manual index calculations (`y * width + x`) inside the loop prevent optimal vectorization.\n**Action:** Use `.chunks_exact_mut(width).take(end_y).skip(start_y)` to safely slice the 1D array per row. Then, use `row[start_x..end_x].fill(f32::INFINITY)` to clear the specific region. This elides inner-loop bounds checks and leverages highly optimized underlying memory operations (`memset`), significantly improving performance.

## [Performance] f32::hypot vs Manual Math
**Learning:** In hot math paths (like vector magnitude calculation), `f32::hypot` calls down to the C math library, introducing expensive underflow/overflow bounds checking that prevents inlining and vectorization.
**Action:** Replace `f32::hypot(x, y)` with `(x*x + y*y).sqrt()`. While this sacrifices protection against intermediate overflow at the extreme limits of f32, it drastically improves execution speed (e.g. ~74% reduction in execution time for `Vec2::length`). Add `#[allow(clippy::imprecise_flops)]` to suppress the warning.

## [Performance] Fast atan2 Mathematical Approximation
**Learning:** In hot pixel loops (like polar coordinate transformations in Kaleidoscope), calling `f32::atan2` introduces significant overhead due to complex branching and high precision calculations.
**Action:** Replace standard `f32::atan2` calls with a fast mathematical approximation `(abs_dx - abs_dy) / (abs_dx + abs_dy)` combined with explicit quadrant adjustments. This reduces the time spent on inverse trigonometric calculations, offering a substantial performance gain in visual-only effects where perfect numerical precision is not required.

**[Performance Optimization: Fixed-Point Raymarching and Row Iteration in Volumetric Lighting]**
**Learning:** In screen-space volumetric lighting algorithms (like God Rays/Crepuscular Rays) that require iterative sub-pixel sampling along a ray, using floating point math inside the inner accumulation loop is extremely slow. Moreover, iterating over coordinates with nested `x`/`y` loops introduces bounds checking overhead.
**Action:** Replace nested loops with `.par_chunks_exact_mut(width).enumerate()` across rows. For the sub-pixel raymarching, calculate step sizes in floating point outside the loop, but convert them to 16.16 fixed-point arithmetic (`(step * 65536.0) as i32`) for the actual accumulation loop. Pre-compute fractional intensity weights as well. This eliminates `f32` overhead per sample, improving execution speeds by ~85%.

## [Performance] f32::hypot vs Manual Math in Post-Processing
**Learning:** In hot pixel loops for post-processing effects (like Vision, Kaleidoscope, Voronoi, and Procedural textures), `f32::hypot` calls down to the C math library, introducing expensive underflow/overflow bounds checking that prevents inlining and vectorization.
**Action:** Replace `f32::hypot(dx, dy)` with `(dx*dx + dy*dy).sqrt()`. While this sacrifices protection against intermediate overflow at the extreme limits of f32, it drastically improves execution speed. Add `#[allow(clippy::imprecise_flops)]` to suppress the warning.
**[Draw List Memory Allocation]
**Learning:** Using a `thread_local!` buffer to avoid temporary allocations is counterproductive if the data must immediately be `.clone()`d to satisfy an owned value requirement (e.g., passing to a struct constructor like `DrawBatch::new`). In these cases, it is more efficient to directly allocate the required `Vec::with_capacity()` or use `.reserve()` on the target collection to avoid the redundant memory copy.
**Action:** When creating new owned `Vec` instances that are immediately moved, prefer direct allocation with `Vec::with_capacity` combined with `spare_capacity_mut()` for uninitialized writes instead of staging through a thread-local buffer that requires a subsequent `.clone()`. Always use `.reserve()` before batch insertions.
**[Eliminate Bounds Checks with iter_mut().enumerate()]
**Learning:** In hot pixel loops that write to exactly-sized slices (like ), manual indexing () incurs per-pixel bounds checks. Replacing this with  provides the index while allowing the compiler to mathematically prove safety and elide bounds checks.
**Action:** Always prefer  when you need both the index and mutable access to a 1D slice or sub-slice.

**[Eliminate Bounds Checks with iter_mut().enumerate()]**
**Learning:** In hot pixel loops that write to exactly-sized slices (like `row`), manual indexing (`for x in 0..width { row[x] = ... }`) incurs per-pixel bounds checks. Replacing this with `row.iter_mut().enumerate()` provides the index while allowing the compiler to mathematically prove safety and elide bounds checks.
**Action:** Always prefer `for (i, p) in row.iter_mut().enumerate()` when you need both the index and mutable access to a 1D slice or sub-slice.

## TileBins Sorting Parallelization Overhead
**Learning:** When sorting tile bins or linked lists with a moderate count (e.g., 1080p tile resolution), sequential sorting (extracting indices to a pre-allocated `Vec`, sorting, and rebuilding the list) can significantly outperform `rayon` parallelization (`par_iter_mut()`) due to thread-spawning overhead and the complexity of bypassing the borrow-checker safely.
**Action:** Do not use `par_iter_mut()` for operations that span many small lists where the actual computational load (e.g., sorting 2-10 items per tile) is dwarfed by the parallelism and synchronization overhead.
**[Optimize Mesh Preparation with `Cow` and `with_capacity`]**
**Learning:** `mesh.normals.clone()` inside of `prepare_lit_mesh_data` and `prepare_textured_mesh_data` caused unnecessary heap allocations when `mesh.normals` was already available. Further, mapping via an iterator chain into `.collect::<Vec<_>>()` forced re-allocations along the way, rather than optimally building up the target `Vec`.
**Action:** Replace `.clone()` with `std::borrow::Cow` to either borrow existing normals or own the generated ones without an unconditional allocation. Replace `.collect::<Vec<_>>()` chains with a manual loop into a `Vec::with_capacity()` to pre-allocate exactly the right size in memory, completely eliminating reallocation overhead and preventing cloning when unnecessary.

**[Performance Optimization: Eliminate redundant allocation when creating Arc from Vec]**
**Learning:** Using `Arc::from(vec.clone().into_boxed_slice())` performs two allocations (one for the temporary `Vec`, one for the `Arc`).
**Action:** Use `Arc::from(vec.as_slice())` to allocate directly into the `Arc` block, avoiding the intermediate allocation entirely.
**[Performance] Slicing to Elide Bounds Checks in 2D Neighborhood Operations**
**Learning:** When performing 2D image convolution or neighborhood operations (like Sobel edge detection) on a flat 1D buffer, manually calculating the absolute index for every pixel in a 3x3 kernel (e.g. `prev_row_offset + x`) prevents LLVM from proving safety, resulting in 9 bounds checks per pixel.
**Action:** Extract explicit 1D slices for the `prev_row`, `curr_row`, and `next_row` *outside* the inner loop (e.g. `&lum_slice[offset..offset+width]`). Inside the loop, access them via `row[x]`. The compiler will recognize `x` is strictly bounded by `width` and safely eliminate all bounds checks, yielding measurable speedups.

**Optimize Rasterizer Inner Loops with `iter_mut().zip`**
**Learning:** Replacing manually unrolled loops that use `unsafe { get_unchecked_mut }` with idiomatic `iter_mut().zip(...)` chains can yield better performance (e.g., ~3% speedup in flat and gouraud rasterization) by allowing LLVM to more effectively auto-vectorize and elide bounds checks safely.
**Action:** Replaced `while i < len { let depth = zb.get_unchecked_mut(i); ... }` with `for (depth, pixel) in zb[i..len].iter_mut().zip(fb[i..len].iter_mut())` in `flat.rs` and `gouraud.rs`.

**[Performance Optimization: Bulk Slicing over Per-Pixel Iteration for Image Distortions]**
**Learning:** For post-processing effects that displace pixels along an axis (like a horizontal "Wobble" effect), iterating pixel-by-pixel, calculating a shifted index, and executing a `clamp(0, width - 1)` operation introduces massive branching and bounds-checking overhead inside the hottest loop.
**Action:** Replace per-pixel loops with bulk slice operations. Use a thread-local single-row buffer (`ROW_BUFFER`) to temporarily hold the row's data. Calculate the exact safe slice ranges for the read and write regions, use `.copy_from_slice()` for the bulk of the shift, and then use `.fill()` on the clamped edge slices. This eliminates all inner loop bounds checking and branching, leveraging highly optimized memory routines for a ~48% reduction in execution time.
