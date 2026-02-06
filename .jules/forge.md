**Math Library Convention**
**Learning:** The math library uses Row Vectors with Post-Multiplication (`v * M`). Transformations are applied Left-to-Right (`v * First * Second`). `Mat4` multiplication `A * B` combines `A` then `B`.
**Action:** When refactoring math code, ensure matrix multiplication order preserves the Left-to-Right application sequence.
# Forge's Journal

## Critical Learnings

**[Tuple Obsession in Pipeline]**
**Learning:** The pipeline was passing raw tuples `(i32, i32, f32)` and manual closures for sorting, leading to duplicated logic and hard-to-read signatures.
**Action:** Extract domain structs like `ScreenPoint` and sorting helpers early to prevent "Primitive Obsession" from spreading.

**[Rasterizer God Module]**
**Learning:** `src/rasterizer.rs` has grown to include texture management, multiple rasterization algorithms (flat, gouraud, textured), and interpolation helpers in a single file > 800 lines. This makes it hard to navigate and refactor isolated parts.
**Action:** Future refactors should prioritize splitting `rasterizer.rs` into submodules: `texture.rs`, `rasterizer/mod.rs` (traits?), and algorithm-specific implementations.
