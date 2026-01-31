# Atlas's Journal

## [Refactor primitives module]
**Tangle:** `primitives.rs` is a "God Module" that mixes low-level 2D rasterization, 3D projection, and high-level lighting logic. This violates cohesion and separation of concerns.
**Blueprint:** Split `primitives.rs` into `rasterizer.rs` (pure 2D drawing mechanics) and `pipeline.rs` (3D rendering pipeline). This decouples mechanism from policy.
**Stability:** Separates 2D/3D domains. Future changes to lighting won't risk breaking line drawing.

## [Extract Scanline Logic]
**Tangle:** `pipeline.rs` contained duplicated edge-walking and interpolation logic across multiple rasterization functions (`fill_triangle_3d`, `fill_triangle_gouraud`). This violated DRY and increased the risk of inconsistent behavior (e.g. one function handled clamping, another didn't).
**Blueprint:** Extracted `ScanlineIter` into `src/pipeline/scanline.rs`. It encapsulates the Y-loop, vertical clamping, and barycentric factor calculation.
**Stability:** Centralized the tricky rasterization math. Fixed potential off-screen iteration inefficiency in Gouraud shading. Enforced DRY.
