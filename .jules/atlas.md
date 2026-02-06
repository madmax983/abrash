# Atlas's Journal

## [Decoupling 3D Rasterization from Pipeline]
**Tangle:** The `pipeline` module was acting as a "God Module" for 3D rendering, handling both high-level lighting/shading logic and low-level scanline rasterization. This coupled the concept of a "Pipeline" tightly to the specific software rasterization implementation, preventing the `rasterizer` module from being the single source of truth for pixel drawing. Additionally, `projection` logic was trapped in `pipeline`, creating potential circular dependencies if `rasterizer` needed to project vertices.

**Blueprint:**
1.  **Relocate Projection:** Move `ScreenPoint` and `project_to_screen` to `math` (the lowest common denominator).
2.  **Unify Rasterization:** Move 3D triangle filling and scanline logic from `pipeline` to `rasterizer`.
3.  **Thin Pipeline:** `pipeline` becomes a coordinator that handles lighting and shading, then delegates drawing to `rasterizer`.

**Stability:** Reduces coupling between rendering stages. `rasterizer` becomes cohesive (2D + 3D drawing). `pipeline` becomes focused on "Shader" logic. acyclic dependency graph is preserved.

## [Extract Texture Module and Unify Math]
**Tangle:** The `rasterizer` module was becoming bloated with `Texture` management logic, mixing resource definition with drawing algorithms. Additionally, the `clipping` module defined a local `Lerp` trait and implementations for common types (`Vec3`, tuples), creating ad-hoc mathematical definitions that were not reusable by other modules without circular dependencies or duplication.

**Blueprint:**
1.  **Extract Texture:** Created `src/texture.rs` to house `Texture` and `FilterMode`. Encapsulated internal state (pixels buffer) behind accessors.
2.  **Unify Math:** Moved the `Lerp` trait and its implementations (including tuple support for `(Vec3, f32)` etc.) to `src/math.rs`, centralizing mathematical operations.

**Stability:** `rasterizer.rs` is now focused purely on drawing algorithms. `Texture` is a standalone resource type. `math.rs` is the single source of truth for linear interpolation.
