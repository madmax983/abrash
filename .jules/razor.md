## [Reduction]
**Bloat:** BspTextures and BspMap single-implementation traits.
**Cut:** Removed the public traits entirely and flattened MockTextures and IntegrationBspMap into concrete structs `BspTextureCache` and `BspMapData` usable by both the main engine and the tests.
**Saved:** Multiple files touched, eliminated two completely unneeded traits and simplified the dependency tree.

## [Reduction]
**Bloat:** The `Lerpable` trait and duplicate generic Bezier/spline curve functions in `curve.rs` vs `math.rs`.
**Cut:** Deleted the single-use `Lerpable` trait entirely and replaced the generic `curve.rs` functions with the concrete `Vec3` versions from `math.rs`.
**Saved:** ~140 lines of code and reduced cognitive overhead of duplicate logic.

## [Reduction]
**Bloat:** `BspTextures` and `BspMap` single-implementation traits used exclusively for test mocks, violating YAGNI and KISS.
**Cut:** Removed the traits entirely, replacing them with concrete `BspTextureCache` and `BspMapData` structs.
**Saved:** Simplified module dependency graph and removed intermediate trait bounds across renderer arguments.

## [Reduction]
**Bloat:** `Evaluable` as a trait with single implementations in `abrash-anim`, which caused dynamic dispatch (`Box<dyn Evaluable<T>>`) throughout the animation sequences, timelines and skeletal clip evaluators.
**Cut:** Replaced `Evaluable` trait with an `enum Evaluable<T>` containing `Keyframe`, `Hold` and `Sequence`. All dynamically dispatched `Box<dyn Evaluable<T>>` occurrences were removed and replaced with concrete enum variants, achieving zero-cost abstractions and keeping memory flat.
**Saved:** Removed the use of dynamic dispatch/`Box` completely across `abrash-anim` and `abrash-skeletal`.
## [Reduction]
**Bloat:** Deeply nested 'Pyramid of Doom' (9+ levels of indentation) duplicated across serial and parallel code paths in `apply_kuwahara`.
**Cut:** Extracted the inner region processing loop into a single flat `process_kuwahara_region` helper function, shared by both execution paths.
**Saved:** Reduced max indentation from 10 levels to 3 levels, eliminated 100+ lines of exact code duplication.
## [Reduction]
**Bloat:** `Lerp` trait in `benches/clipping_optimization.rs`
**Cut:** Removed the single-implementation trait and used `impl Fn` directly in the legacy function signature for the benchmark.
**Saved:** Removed unnecessary generic indirection, deleted ~20 lines of trait definition/implementation boilerplate.

## [Reduction]
**Bloat:** `MeshValidationError` enum in `crates/abrash-gpu-render/src/lib.rs`
**Cut:** Replaced the multi-variant enum with `Result<(), String>` returning formatted error strings directly, since it was only ever used to print errors anyway.
**Saved:** Deleted ~20 lines of enum definition and `Display` implementation boiler plate.

## [Reduction]
**Bloat:** `from_i32_trait` and `into_i32_trait` tests in `crates/abrash-core/src/fixed16_16.rs`.
**Cut:** Renamed to `from_i32` and `into_i32` to remove the confusing and unnecessary `_trait` suffix since they just tested `From`/`Into` which are standard language features, not custom domain traits.
**Saved:** Improved clarity and cognitive load.

## [Reduction]
**Bloat:** `FrameClock` in `src/platform/winit.rs`.
**Cut:** Deleted the struct and used `Instant::now()` and `Instant::duration_since()` directly inside the `run_windowed` event loop, eliminating an unnecessary struct wrapper for just two lines of logic.
**Saved:** Removed the struct, impl block, and `Default` impl (25 lines of code) and simplified the main loop state.
