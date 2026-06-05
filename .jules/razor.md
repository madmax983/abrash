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
**Bloat:** `TimelineState<T>` enum in `crates/abrash-anim/src/timeline.rs` tracking `Playing` and `Completed` state, adding an extra enum abstraction that was overlapping with the existing `last_sample: Sample<T>` field.
**Cut:** Removed the `TimelineState<T>` enum entirely and replaced it with a simple `is_completed: bool` flag on the `Timeline` struct.
**Saved:** Removed 1 enum definition and simplified state machine logic.

## [Reduction]
**Bloat:** `PoolEntry<T>` enum in `crates/abrash-render/src/render_api/handles.rs` using `Occupied` and `Vacant` variants, requiring `unreachable!()` panic branches to handle state invariants inside `match` blocks.
**Cut:** Flattened the `PoolEntry<T>` enum into a single struct containing an `Option<T>` and a generation counter. Replaced complex `match` blocks with flat `if let` / `filter` logic.
**Saved:** Eliminated 1 enum definition, removed `unreachable!()` panics, flattened the `ResourcePool` memory model and simplified `remove`/`insert` logic.
