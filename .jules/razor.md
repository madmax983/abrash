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
**Bloat:** `RefractionEncodeArgs` was passed as 9 separate arguments causing the `too_many_arguments` lint. The `from_gpu` setup method had `too_many_lines`. `map_or_else` was unused.
**Cut:** Grouped parameters into `RefractionEncodeArgs` struct, used `map_or_else` instead of an `if let/else` expression, pulled out buffer creation to a helper `create_buffers` method. Fixes `clippy::too_many_arguments`, `clippy::too_many_lines`, `clippy::option_if_let_else`, `clippy::needless_pass_by_value`, and doc lints.
**Saved:** Multiple complex functions streamlined, significantly reducing argument lists.
