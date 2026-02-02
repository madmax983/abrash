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
**Bloat:** Entire 2D `rasterizer` and `shapes` modules (and associated `Vec2`/`Mat2` types) in a dedicated 3D engine project.
**Cut:** Deleted `src/rasterizer.rs`, `src/shapes.rs`, `tests/rasterizer_tests.rs`, `tests/shapes_tests.rs`, `examples/rotating_polygon.rs`, `examples/filled_primitives.rs`, `benches/primitives.rs`, and removed `Vec2`/`Mat2` from `src/math.rs`.
**Saved:** ~700 lines of code, 2 modules, 2 example files, 1 benchmark file, and reduced compile times/binary size.
