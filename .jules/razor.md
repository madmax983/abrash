# Razor's Journal

## [Reduction]
**Bloat:** Deprecated `Vec3::zero` and `Mat4::mul` methods that were just wrappers or old implementations.
**Cut:** Removed methods and moved `Mat4` multiplication logic directly into `Mul` trait implementation.
**Saved:** 20 lines of code and reduced API surface area.

## [Reduction]
**Bloat:** `experimental` folder with zombie code (`particles.rs`) and deep hierarchy for `post_process.rs`. Duplicated `Event` and `WindowError` types in platform backends.
**Cut:** Deleted `particles.rs`, moved `post_process.rs` to root, consolidated platform types in `platform/mod.rs`.
**Saved:** ~100 lines of dead code, 1 directory level, and removed DRY violation in platform layer.

## [Reduction]
**Bloat:** `src/pipeline` and `src/experimental` directories. `pipeline` was just a wrapper around 3D rasterization logic, separate from 2D rasterizer. `experimental` was dead code.
**Cut:** Moved projection to `math.rs`, 3D rasterization to `rasterizer.rs`. Deleted `pipeline` and `experimental`.
**Saved:** ~2 directories, 3 files, significantly flattened structure.
