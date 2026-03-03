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
**Bloat:** `src/pipeline` directory with dead code (duplicate of `rasterizer.rs`). Unused experimental modules (`particles.rs`, `fog.rs`, `modifiers.rs`). Misnamed test file (`pipeline_tests.rs`).
**Cut:** Deleted `src/pipeline` and unused experimental files. Renamed `tests/pipeline_tests.rs` to `tests/rasterizer_3d_tests.rs`.
**Saved:** ~500 lines of dead code and a directory. Reduced confusion by removing duplicate logic.

## [Reduction]
**Bloat:** `src/light.rs` containing only one helper function and one unused function. `dummy` naming for TUI backend.
**Cut:** Moved `color_to_u32` to `src/rasterizer.rs`, deleted `src/light.rs`, and renamed `src/platform/dummy.rs` to `src/platform/tui.rs`.
**Saved:** 1 file, ~15 lines, and reduced cognitive load by using explicit names.

## [Reduction]
**Bloat:** `src/experimental` directory containing mixed used and unused modules. `pixel_sort`, `reflection`, `svg_renderer`, `terrain`, `text`, `vhs` were unused or dead code.
**Cut:** Deleted unused modules. Moved used modules (`ascii`, `heat_vision`, `particles`, `procedural`, `skybox`) to `src/` root. Deleted `src/experimental` directory.
**Saved:** 6 files (~1000 lines of dead code), 1 directory level, and flattened module hierarchy.

## [Reduction]
**Bloat:** `Vec2Ext` trait with single method `length` and `WindowBackend` trait with only conditionally-compiled concrete implementations.
**Cut:** Deleted `Vec2Ext`, moved `length()` to `Vec2` struct. Deleted `WindowBackend` trait, converted `impl WindowBackend` blocks to inherent `impl` blocks.
**Saved:** ~50 lines of code, reduced abstraction layers, and enforced KISS.
