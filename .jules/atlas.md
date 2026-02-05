# Atlas's Journal

## [Decoupling 3D Rasterization from Pipeline]
**Tangle:** The `pipeline` module was acting as a "God Module" for 3D rendering, handling both high-level lighting/shading logic and low-level scanline rasterization. This coupled the concept of a "Pipeline" tightly to the specific software rasterization implementation, preventing the `rasterizer` module from being the single source of truth for pixel drawing. Additionally, `projection` logic was trapped in `pipeline`, creating potential circular dependencies if `rasterizer` needed to project vertices.

**Blueprint:**
1.  **Relocate Projection:** Move `ScreenPoint` and `project_to_screen` to `math` (the lowest common denominator).
2.  **Unify Rasterization:** Move 3D triangle filling and scanline logic from `pipeline` to `rasterizer`.
3.  **Thin Pipeline:** `pipeline` becomes a coordinator that handles lighting and shading, then delegates drawing to `rasterizer`.

**Stability:** Reduces coupling between rendering stages. `rasterizer` becomes cohesive (2D + 3D drawing). `pipeline` becomes focused on "Shader" logic. acyclic dependency graph is preserved.

## [Texture Extraction & Lighting Simplification]
**Tangle:** `src/rasterizer.rs` was becoming bloated with unrelated concerns: it defined `Texture` resource logic (loading, sampling) alongside low-level drawing primitives. Additionally, `fill_triangle_lit` suffered from an "Argument Jungle" (10 arguments), mixing geometry, material, and lighting configuration in a single call.

**Blueprint:**
1.  **Extract Texture:** Moved `Texture` and `FilterMode` to a new `src/texture.rs` module. `rasterizer` now consumes `Texture` rather than owning the definition.
2.  **Encapsulate Lighting:** Introduced `DirectionalLight` struct in `src/light.rs` to bundle direction and color.
3.  **Clean API:** Refactored `fill_triangle_lit` to accept `&DirectionalLight`, reducing argument count and enforcing domain boundaries.

**Stability:** Improved cohesion in `rasterizer` (focused on drawing). `texture` is now a standalone resource module. Lighting API is more ergonomic and extensible.
