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
## std::mem::take Reusable Buffer Performance Regression
**Learning:** Using  to extract a  or  from a reusable buffer (like  storage) destroys the capacity of the stored buffer, forcing costly heap reallocations on subsequent uses and causing massive performance regressions (e.g., >90% slowdown).
**Action:** Instead, extract the result via  and use  to retain the buffer's capacity for future iterations.

## .collect() is Optimized
**Learning:** In Rust,  already optimally leverages  and internal traits (like  or ) to pre-allocate memory. Manually replacing  with  followed by  or a  loop is an anti-pattern that achieves no performance gain and degrades code readability.
**Action:** Rely on  when transforming iterators into vectors unless profiling strictly proves an unoptimized iterator path.
## std::mem::take Reusable Buffer Performance Regression
**Learning:** Using `std::mem::take()` to extract a `String` or `Vec` from a reusable buffer (like `thread_local!` storage) destroys the capacity of the stored buffer, forcing costly heap reallocations on subsequent uses and causing massive performance regressions (e.g., >90% slowdown).
**Action:** Instead, extract the result via `.clone()` and use `.clear()` to retain the buffer's capacity for future iterations.

## .collect() is Optimized
**Learning:** In Rust, `Iterator::collect::<Vec<_>>()` already optimally leverages `size_hint()` and internal traits (like `TrustedLen` or `ExactSizeIterator`) to pre-allocate memory. Manually replacing `.collect()` with `Vec::with_capacity(iter.size_hint().0)` followed by `.extend()` or a `for` loop is an anti-pattern that achieves no performance gain and degrades code readability.
**Action:** Rely on `.collect()` when transforming iterators into vectors unless profiling strictly proves an unoptimized iterator path.
