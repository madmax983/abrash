## [Reduction]
**Bloat:** `TileRenderer` (Complex, SIMD-heavy, binned renderer intended for high-res optimizations but unused in examples and complex to maintain).
**Cut:** Removed `TileRenderer` and its dependency `HiZBuffer`. Simplified `Scene::render` to use `fill_triangle_3d` directly.
**Saved:** ~1500 lines of code. Reduced complexity of scene rendering pipeline. Removed "speculative generality".

## [Reduction]
**Bloat:** `HiZBuffer` (Hierarchical Z-Buffer implementation that was noted as slower than scalar and complex).
**Cut:** Deleted entirely.
**Saved:** ~500 lines of code.

## [Refactor]
**Bloat:** `heat_vision.rs` in root source directory.
**Cut:** Moved to `src/post_process/heat_vision.rs`.
**Saved:** Flattened root directory structure (slightly).
