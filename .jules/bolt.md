**[Replacing SmallVec with a persistent Vec]**
**Learning:** In hot paths handling variable workloads (e.g., sorting triangles per-tile), locally scoped `SmallVec`s can still spill to the heap if their capacity is exceeded, incurring allocation overhead.
**Action:** Replace them with a persistent, reusable `Vec` stored on the parent struct (e.g., `sort_workspace: Vec<u32>`) and clear it on each use to guarantee zero dynamic heap allocations across frames.
