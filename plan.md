1. Add `thread_local!` buffers to `src/experimental/swirl.rs`, `src/experimental/chromatic_aberration.rs`, `src/experimental/crepuscular.rs`, `src/experimental/sharpen.rs`, and `src/experimental/water_ripple.rs` to replace `.to_vec()` heap allocations on every frame.
2. Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
3. Submit the change with "⚡ Bolt: [performance improvement] Eliminate per-frame memory allocation in post-processing filters"
