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
**Bloat:** The `CoordMode` enum in `crates/abrash-gpu-render/src/blitter.rs` which was unused and flagged with `#[allow(dead_code)]`.
**Cut:** Deleted the unused `CoordMode` enum completely according to the YAGNI principle.
**Saved:** Removed 6 lines of unused code.

## [Reduction]
**Bloat:** The `dielectric_f0` field in `PbrUniforms` within `crates/abrash-render/src/rasterizer/pbr.rs` which was unused and flagged with `#[allow(dead_code)]`.
**Cut:** Deleted the unused `dielectric_f0` field completely according to the YAGNI principle.
**Saved:** Removed 3 lines of unused code struct declarations and initializations.

## [Reduction]
**Bloat:** The `playback` field in `SkeletonAnimator` within `crates/abrash-skeletal/src/animator.rs` which was unused and flagged with `#[allow(dead_code)]`.
**Cut:** Deleted the unused `playback` field completely according to the YAGNI principle.
**Saved:** Removed 2 lines of unused code.
