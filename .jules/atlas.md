# Atlas's Journal

## [Refactor primitives module]
**Tangle:** `primitives.rs` is a "God Module" that mixes low-level 2D rasterization, 3D projection, and high-level lighting logic. This violates cohesion and separation of concerns.
**Blueprint:** Split `primitives.rs` into `rasterizer.rs` (pure 2D drawing mechanics) and `pipeline.rs` (3D rendering pipeline). This decouples mechanism from policy.
**Stability:** Separates 2D/3D domains. Future changes to lighting won't risk breaking line drawing.

## [Modularize pipeline and deduplicate projection]
**Tangle:** `pipeline.rs` was becoming a Blob, mixing projection math with rasterization loops. `experimental/particles.rs` duplicated projection logic (The Sprawl).
**Blueprint:** Converted `pipeline.rs` to a directory module. Extracted `projection.rs` for shared coordinate transformations. Extracted `draw_scanline_gouraud` to simplify `fill_triangle_gouraud`.
**Stability:** Centralized projection logic reduces bugs. Smaller functions in pipeline improve maintainability.
