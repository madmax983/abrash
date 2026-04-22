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
