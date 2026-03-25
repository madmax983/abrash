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
**Bloat:** The `AsciiExporter` single-implementation trait in `src/experimental/ascii_export.rs`.
**Cut:** Removed the trait and `ascii_export.rs` entirely. Moved the implementations of `export_txt` and `export_ansi` directly to a concrete `impl Framebuffer` block in `src/framebuffer.rs`.
**Saved:** Removed 1 file, eliminated the trait abstraction, and lowered cognitive overhead by attaching the methods directly to the struct they operate on.

## [Reduction]
**Bloat:** `FastU64Builder` a custom hasher builder trait implementation specifically to instantiate `FastU64Hasher` for `HashMap`.
**Cut:** Removed `FastU64Builder` entirely and replaced it with `std::hash::BuildHasherDefault<FastU64Hasher>`.
**Saved:** ~15 lines of redundant builder abstraction and boilerplate code.

## [Reduction]
**Bloat:** `FastSplitter` custom trait implementation in `src/obj_loader.rs` created for parsing `v/vt/vn` obj vertices.
**Cut:** Removed the `FastSplitter` abstraction and replaced it with standard library string `split('/')` parsing methods.
**Saved:** ~30 lines of boilerplate code, improving maintainability with no regressions.
**Bloat:** `HiZOcclusion` and `HiZPyramidWriter` single-use adapter traits in `crates/abrash-gpu/src/d3d12_binning.rs` used to bridge methods from `HiZBuffer` in the `abrash` crate.
**Cut:** Deleted the traits and the wrapper struct implementations (`HiZOcclusionAdapter`, `HiZPyramidWriterAdapter`) in `src/gpu/mod.rs`. Replaced trait parameters with direct closure parameters (`impl Fn` and `impl FnMut`).
**Saved:** ~30 lines of boilerplate and removed unnecessary abstractions between crates.

## [Reduction]
**Bloat:** Unnecessary struct padding fields named `_pad` causing `clippy::pub_underscore_fields` warnings in `crates/abrash-gpu-render` and complex `match` branches in `renderer.rs`.
**Cut:** Renamed `_pad` fields to `pad` to follow YAGNI, consolidated identical match arms, simplified map_or closures to `is_none_or`, and removed verbose format string arguments in examples.
**Saved:** Eliminated 30+ clippy warnings and simplified several match and map_or closures.

## [Reduction]
**Bloat:** The `AsciiExporter` single-implementation extension trait in `crates/abrash-render/src/experimental/ascii_export.rs`.
**Cut:** Deleted the trait and its file. Moved the `export_ascii` and `export_ansi` methods directly to the local `AsciiConverter` struct in `crates/abrash-render/src/ascii.rs`.
**Saved:** Eliminated 1 trait, 1 file (`ascii_export.rs`), and consolidated the ASCII export API into a single, straightforward struct without unnecessary abstraction overhead.
