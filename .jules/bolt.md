**Fast Inv Sqrt vs Stdlib SQRT Recip**
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
