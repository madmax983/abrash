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
**Bloat:** The `Vec2Ext` trait in `src/experimental/sdf.rs` which was only implemented for `Vec2` to provide a `length()` method.
**Cut:** Removed the `Vec2Ext` trait entirely. Moved the `length()` method directly into the `impl Vec2` block in `src/math.rs`.
**Saved:** 10 lines of trait boilerplate, flattened the abstraction, and unified the API for `Vec2`.

## [Reduction]
**Bloat:** The `WindowBackend` trait which abstracted window operations but was only implemented by `Win32Window` and `TuiWindow` (which are conditionally compiled).
**Cut:** Deleted the `WindowBackend` trait, moved its methods directly to `impl Win32Window` and `impl TuiWindow`, and updated examples to rely purely on the concrete type alias `Window`.
**Saved:** ~10 lines of trait boilerplate, removed the need for users to import a trait just to use the window, and flattened the platform abstraction.

## [Reduction]
**Bloat:** The `AabbBounds` trait in `tile.rs` which was only used as a constraint for a single helper function (`clear_tile_bounds`).
**Cut:** Deleted the `AabbBounds` trait and its three implementations. Replaced the trait constraint with an explicit closure parameter `get_bounds: impl Fn(&T) -> (i32, i32)`.
**Saved:** 30 lines of code and simplified the mental model by passing data directly rather than funneling it through a single-use trait.

## [Reduction]
**Bloat:** The `Lerp` trait in `clipping.rs` which abstracted generic linear interpolation math over simple tuples and primitives.
**Cut:** Deleted the `Lerp` trait entirely. Replaced its usage in clipping functions with explicit `lerp: impl Fn(V, V, f32) -> V` closure arguments. Added inherent `lerp` methods to `Vec2` and `Vec4`.
**Saved:** 100 lines of repetitive trait implementation blocks. Math interpolation is now explicit and localized to the caller.

## [Reduction]
**Bloat:** `WindowBackend` trait which had only two implementations and added unnecessary abstraction layers.
**Cut:** Removed the trait and replaced it with a direct type alias `Window` depending on features (`tui::TuiWindow` or `win32::Win32Window`).
**Saved:** ~30 lines of code, simplified platform module, reduced trait bounds.

## [Reduction]
**Bloat:** `Lerp` trait used exclusively for simple interpolation during frustum clipping.
**Cut:** Replaced the trait bound with an explicit closure parameter (`lerp: impl Fn(V, V, f32) -> V`) in clipping functions.
**Saved:** 50 lines of code, removed unnecessary trait boilerplate in tests and codebase.

## [Reduction]
**Bloat:** `FastU64Builder` which wrapped `FastU64Hasher` and implemented `BuildHasher`.
**Cut:** Eliminated the wrapper builder struct where possible and used the hasher directly.
**Saved:** 15 lines of code, removed an unnecessary layer of indirection.
