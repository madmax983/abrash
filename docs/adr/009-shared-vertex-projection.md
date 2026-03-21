# ADR-009: Shared Vertex Projection for Frustum-Interior Meshes

## Status

Accepted

## Context

Profiling the scene render pipeline (100 objects, 20K triangles, 640x480) revealed that
`submit_mesh` dominated frame time at ~256us (83% of total). The hot path was
`prepare_triangle_static` called 20K times, each performing:

1. `clip_triangle_to_frustum` (SSE trivial accept/reject + Sutherland-Hodgman)
2. `project_triangle_to_screen` (SSE perspective divide + NDC-to-screen)
3. Backface cull, Y-sort, dz/dx gradient, AABB computation

The profiling breakdown:

| Phase            | Time   | % of total |
|------------------|--------|------------|
| fb/zb clear      | ~44us  | 14%        |
| scene.extract()  | ~8us   | 3%         |
| submit_mesh      | ~256us | 83%        |
| end_frame (bin+render+merge) | ~0us | <1% |

Key observation: with 121 vertices per mesh and 200 triangles, each vertex was being
clipped and projected ~5x redundantly (once per triangle it appears in). For meshes
fully inside the view frustum (the common case), clipping produces no new vertices --
the per-triangle clip call was pure overhead.

## Decision

Add a fast path in `submit_mesh` that detects when ALL vertices of a mesh are inside
the view frustum (`-w <= x,y,z <= w` and `w > 0`). When true:

1. **Pre-project all vertices once** to screen space (121 vs 600 projections per mesh)
2. **Skip `clip_triangle_to_frustum` entirely** for all triangles in the mesh
3. Per-triangle: just backface test + Y-sort + gradient setup + push to prepared list

When any vertex is outside the frustum, fall back to the existing per-triangle
clip+project pipeline.

## Measurements

| Benchmark              | Before  | After   | Change |
|------------------------|---------|---------|--------|
| scene_render (total)   | ~309us  | ~198us  | **-36%** |
| submit_mesh_only       | ~256us  | ~124us  | **-52%** |
| rasterize_only         | ~251us  | ~119us  | **-53%** |

Per-triangle cost: 12.8ns -> ~6.2ns

## Consequences

**Positive:**
- 36% faster scene rendering for the common case (objects inside frustum)
- No API changes required
- Graceful fallback: meshes crossing frustum boundaries use the existing path
- Zero overhead for the fallback path (single branch on `all_inside`)

**Negative:**
- ~50 lines of duplicated triangle setup code (backface, sort, dz/dx, AABB)
- The `all_inside` scan is O(vertices) per mesh -- negligible for typical meshes
  but could matter for meshes with 10K+ vertices (still faster than per-triangle clipping)
- Only optimizes the sequential (non-parallel) path; the parallel path still uses
  per-triangle clipping via `par_extend`

**Not addressed:**
- The Vec allocation for pre-projected vertices in `submit_mesh_unclipped` could be
  eliminated with a thread-local scratch buffer if it becomes a bottleneck
- The parallel path could be similarly optimized with per-object frustum classification
