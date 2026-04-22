**[O(1) Array Lookups vs O(N) Vec::contains]**
**Learning:** In algorithms like topological sorting, checking if an element has been processed using `Vec::contains` inside a loop results in an O(N^2) bottleneck. Replacing this with a pre-allocated boolean vector (`vec![false; N]`) for O(1) lookups provides massive speedups (e.g., 10x) for large datasets.
**Action:** Replace `!sorted.contains(&i)` with an O(1) boolean vector lookup when iterating over elements.

**[Eliminate bounds check panics with min/max chaining]**
**Learning:** Replacing `.clamp(min, max)` with `.max(min).min(max)` on integers provides zero performance benefit, as LLVM optimizes both to the exact same assembly instructions. Furthermore, this anti-pattern triggers the `clippy::manual_clamp` lint.
**Action:** Do not replace `clamp` with `.max(min).min(max)` on integers for performance.

**[Cache expensive Mathematical Look-Up Tables]**
**Learning:** In post-processing filters, precomputing mathematical Look-Up Tables (LUTs) using expensive operations like `f32::powf` on every frame call is a major bottleneck. Caching these LUTs per-thread (e.g., using `thread_local!` and `std::cell::RefCell`) for mostly-static parameters like gamma or contrast eliminates per-frame recalculations and provides significant speedups without modifying function signatures.
**Action:** Use `thread_local!` and `std::cell::RefCell` to cache static configurations in rendering filters to eliminate unnecessary loop calculations.

**[Workspace vs Child Crate Dependencies]**
**Learning:** Inherited workspace dependencies (like `foldhash`) cannot be used inside individual child crates without explicitly adding them to that specific crate's `Cargo.toml`. If a rule forbids modifying `Cargo.toml` without instruction, optimizing via such dependencies is blocked and must be avoided.
**Action:** Evaluate dependencies within the exact scope of the sub-crate `Cargo.toml` rather than the workspace root `Cargo.toml` when determining if a crate can be used.

**[Elide per-frame object allocation for Rendering]**
**Learning:** Re-allocating the `DrawList` array containing transformed geometry vertices and draw calls incurs massive dynamic heap allocation penalties each frame when rendering hundreds of objects. By utilizing `thread_local!` `RefCell` storage to cache and reclaim the intermediate buffers between frames in `CpuRenderer` and `Scene`, we eliminate the per-frame allocations entirely, resulting in measurable performance improvements in `scene_render`.
**Action:** Cache intermediate rendering vectors via `thread_local!` and reuse them rather than constructing a new `DrawList::with_capacity` dynamically on every `extract_draw_list` call.
## [Prevent Allocator Resizing Chains in Iterator Maps]
**Learning:** Replacing `.collect::<Vec<_>>().` with `Vec::with_capacity(n)` followed by `.extend(...)` prevents intermediate allocator resizing chains. This is particularly effective when `ExactSizeIterator` optimizations for complex iterator mapping fail to inline optimally in the frontend.
**Action:** Use `Vec::with_capacity` and `extend` instead of `.collect()` for known-size iterators doing complex maps.
## 2024-04-22 - [TileRenderer set_clear_color removal optimization]
**Learning:** Adding `set_clear_color` into `TileRenderer` and then resetting it after clears in `CpuRenderer` creates performance regressions, especially in `scene_render_integrated_clear_100_objects`. The `CpuRenderer` relies heavily on tight inner loops, and modifying global clear colors each frame defeats certain tile-clearing optimizations.
**Action:** Reverting the `set_clear_color` addition and the usage of `clone()` during vertex range slicing (which allocated per batch) yielded measurable performance improvements in the `submit_mesh_only_20k_tris` and `4k_20k_tris_100obj` benchmarks.
## [fast_sin_cos optimization]
**Learning:** Floating-point `.round()` operations in hot inner loops are slow. Replacing them with fast integer casting logic (e.g., shifting values to be positive, casting to `i32`, then back to `f32` like `((val + 16384.5) as i32 as f32 - 16384.0)`) avoids branching and yields measurable performance improvements.
**Action:** Replaced `.round()` with fast integer casting logic in `fast_sin_cos` function inside `abrash-core/src/math.rs`.
## [Voronoi Distance Calculation]
**Learning:** In distance-based algorithms like Voronoi diagrams, evaluating configuration branches (e.g., determining which metric to use) inside nested per-pixel and per-seed loops is highly inefficient. Furthermore, computing expensive square roots per-seed is unnecessary when simply finding the minimum distance.
**Action:** Hoist conditional checks for  out of the inner loop and into variables (, etc.). For Euclidean distance, defer the  operation until after the loop by directly comparing squared distances (), which yields a ~69% speedup.
## [Voronoi Distance Calculation]
**Learning:** In distance-based algorithms like Voronoi diagrams, evaluating configuration branches (e.g., determining which metric to use) inside nested per-pixel and per-seed loops is highly inefficient. Furthermore, computing expensive square roots per-seed is unnecessary when simply finding the minimum distance.
**Action:** Hoist conditional checks for `metric` out of the inner loop and into variables (`is_euclidean`, etc.). For Euclidean distance, defer the `sqrt()` operation until after the loop by directly comparing squared distances (`dx*dx + dy*dy`), which yields a ~69% speedup.
**[Ellipse algebraic simplification]
**Learning:** In the setup for Region 2 of Bresenham's ellipse algorithm, simplifying the decision variable mathematically (e.g., `ry_sq * x * (x + 1) + rx_sq * ((y - 1)^2 - ry_sq)`) and hoisting type conversions eliminates redundant operations.
**Action:** Applied algebraic simplification to the `p2` initialization in `ellipse.rs`, reducing per-ellipse math operations without altering behavior.
**[Eliding Sqrt in Neon Outline Filter]**
**Learning:** In edge-detection filters (like Sobel in `neon_outline`), calculating the exact magnitude using `sqrt()` for every pixel is a severe bottleneck.
**Action:** Pre-calculate a squared threshold outside the loop (using `u64` to prevent overflow) and compare it against the squared magnitude (`gx*gx + gy*gy`). This safely elides the expensive `sqrt()` and floating-point cast operations for the vast majority of non-edge pixels.

**[Enum Swap Redundancy]**
**Learning:** Using `std::mem::replace` multiple times in nested enum matches inside hot paths creates unnecessary temporary values and stack shuffling, even if logically safe.
**Action:** Extract generation states outside the match block first, then do a single `std::mem::replace` with the computed state, significantly accelerating tight resource-reclamation loops.
