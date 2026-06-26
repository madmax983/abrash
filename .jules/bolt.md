**[Anaglyph Temporary Row Buffer Allocation]**
**Learning:** Allocating temporary buffers (e.g. `vec![0u32; width]`) within the body of a frame-level post-processing function introduces an unnecessary `O(W)` allocation overhead on every single frame rendering cycle.
**Action:** Lift per-frame temporary allocations into a `std::thread_local!` containing a `RefCell`, resizing it to the necessary width once.

**[Anaglyph Temporary Row Buffer Allocation]**
**Learning:** Allocating temporary buffers (e.g. `vec![0u32; width]`) within the body of a frame-level post-processing function introduces an unnecessary `O(W)` allocation overhead on every single frame rendering cycle.
**Action:** Lift per-frame temporary allocations into a `std::thread_local!` containing a `RefCell`, resizing it to the necessary width once.

**[Anaglyph Temporary Row Buffer Allocation]**
**Learning:** Allocating temporary buffers (e.g. `vec![0u32; width]`) within the body of a frame-level post-processing function introduces an unnecessary `O(W)` allocation overhead on every single frame rendering cycle.
**Action:** Lift per-frame temporary allocations into a `std::thread_local!` containing a `RefCell`, resizing it to the necessary width once.

**[Anaglyph Temporary Row Buffer Allocation]**
**Learning:** Allocating temporary buffers (e.g. `vec![0u32; width]`) within the body of a frame-level post-processing function introduces an unnecessary `O(W)` allocation overhead on every single frame rendering cycle.
**Action:** Lift per-frame temporary allocations into a `std::thread_local!` containing a `RefCell`, resizing it to the necessary width once.
