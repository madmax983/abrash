# Atlas's Journal

## [Refactor primitives module]
**Tangle:** `primitives.rs` is a "God Module" that mixes low-level 2D rasterization, 3D projection, and high-level lighting logic. This violates cohesion and separation of concerns.
**Blueprint:** Split `primitives.rs` into `rasterizer.rs` (pure 2D drawing mechanics) and `pipeline.rs` (3D rendering pipeline). This decouples mechanism from policy.
**Stability:** Separates 2D/3D domains. Future changes to lighting won't risk breaking line drawing.
