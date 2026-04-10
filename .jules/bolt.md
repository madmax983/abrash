<<<<<<< SEARCH
**[Pre-allocate HashMaps to eliminate dynamic heap reallocations]**
**Learning:** Using `HashMap::new()` in large iterations or when dealing with known data sizes (like parsing glTF joints and nodes) results in unnecessary dynamic heap reallocations and creates empty maps that scale inefficiently during heavy insertions.
**Action:** Pre-allocated HashMaps using `HashMap::with_capacity()` utilizing known bounds from iterators and slices, eliminating reallocation overhead on the hot parsing path.

**[Thread-local buffer caching with Rayon]**
**Learning:** When using a `thread_local!` `RefCell` to cache buffers for use with Rayon parallel closures (e.g., `.par_chunks_exact_mut()`), you must always extract a primitive slice (e.g., `let src = src_pixels.as_slice()`) *before* the closure. Attempting to capture or access the `RefMut` directly inside the parallel closure will cause compilation failures because `RefMut` is `!Send`.
**Action:** Implemented caching for `apply_frosted_glass` to remove `.to_vec()` dynamic allocations, capturing the primitive slice correctly before launching the parallel iterator.
=======
**[Pre-allocate HashMaps to eliminate dynamic heap reallocations]**
**Learning:** Using `HashMap::new()` in large iterations or when dealing with known data sizes (like parsing glTF joints and nodes) results in unnecessary dynamic heap reallocations and creates empty maps that scale inefficiently during heavy insertions.
**Action:** Pre-allocated HashMaps using `HashMap::with_capacity()` utilizing known bounds from iterators and slices, eliminating reallocation overhead on the hot parsing path.

**[Thread-local buffer caching with Rayon]**
**Learning:** When using a `thread_local!` `RefCell` to cache buffers for use with Rayon parallel closures (e.g., `.par_chunks_exact_mut()`), you must always extract a primitive slice (e.g., `let src = src_pixels.as_slice()`) *before* the closure. Attempting to capture or access the `RefMut` directly inside the parallel closure will cause compilation failures because `RefMut` is `!Send`.
**Action:** Implemented caching for `apply_frosted_glass` to remove `.to_vec()` dynamic allocations, capturing the primitive slice correctly before launching the parallel iterator.
>>>>>>> REPLACE
