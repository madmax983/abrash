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
**Bloat:** `TimelineState` enum with only 2 variants (`Playing` and `Completed`), the latter of which just redundantly held a clone of the final sample that's already stored in `Timeline::last_sample`.
**Cut:** Removed the `TimelineState` enum completely. Replaced it with a simple `is_completed: bool` flag and relied entirely on `last_sample` to cache the final evaluated value.
**Saved:** Eliminated unnecessary state-matching boilerplate, memory overhead for the redundant sample, and an entire enum definition.

## [Reduction]
**Bloat:** `PoolEntry` enum with `Occupied` and `Vacant` variants caused verbose `match` destructuring, `unreachable!()` panics in the free list logic, and confusing generation swaps in `std::mem::replace`.
**Cut:** Flattened `PoolEntry` into a simple struct with an `Option<T>` for the value and a `generation` integer. The `Vacant` state is simply `value: None`, removing the need for a separate enum variant.
**Saved:** Removed the `PoolEntry` enum, several verbose `match` blocks, and a dedicated `unreachable!()` panic test for removing items in a vacant state.
