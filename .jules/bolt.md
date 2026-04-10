**Fast Inv Sqrt vs Stdlib SQRT Recip**
**Learning:** Using `fast_inv_sqrt` (Quake III trick) is slower and less precise than using `std`'s `sqrt().recip()` directly on modern CPU architectures when compiling. The standard library leverages hardware-accelerated instructions (like `rsqrtss`) automatically and provides better results.
**Action:** Replaced `fast_inv_sqrt(dist_sq)` with `dist_sq.sqrt().recip()` in the scalar fallback paths of point-lit and shadowed phong rasterizers, resulting in ~6-7% performance improvement in single-point light rendering.

**[Learning from `abrash-render` Tile Renderer]**
**Learning:** Hoisting dynamic allocations out of hot loops entirely using `thread_local!` combined with `RefCell<Vec<T>>` can be extremely powerful in Rust, but introduces hidden `thread_local` overhead and is completely unnecessary if the collection can simply be lifted out to be state inside the containing struct itself. If avoiding `Vec::new()`, try to store the `Vec` in `self` and use `vec.clear(); vec.extend(...)` where possible to keep the lifetime semantics and API clean.
**Action:** When evaluating allocation hoisting, first check if the structure (`self`) can just own the buffer natively.
