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
