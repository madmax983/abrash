# ADR-010: Tile-Integrated Framebuffer Clear

## Status

Accepted

## Context

After ADR-009 (shared vertex projection, -36%), profiling showed the separate
`fb.clear()` + `zb.clear()` was the second-largest cost at ~43us (24% of the
~179us frame). This is a 2.46 MB cold-cache sequential memset (307K pixels x
4 bytes x 2 buffers) that doesn't fit in L1/L2 and pollutes the cache before
rendering starts.

The tile renderer already clears each non-empty tile's 8 KB buffer in L1 before
rasterizing into it. The insight: extend this to cover ALL tiles, eliminating
the separate full-frame clear entirely.

## Decision

Add an opt-in `clear_color: Option<u32>` to `TileRenderer`. When set, `end_frame()`
writes every tile to the framebuffer:

- **Non-empty tiles**: Clear full tile buffer (all 32 rows) with clear color, rasterize
  triangles, merge full tile to framebuffer
- **Empty tiles**: Write clear color + infinity depth directly to the tile's framebuffer
  region via `merge_empty_tile()`

This replaces one 2.46 MB cold-cache memset with 300 x 8 KB L1-friendly tile writes.
The `CpuRenderer::execute_draw_list()` was updated to use `set_clear_color()` instead
of `target.clear()`.

## Measurements

| Benchmark | Before | After | Change |
|-----------|--------|-------|--------|
| scene_render (with explicit clear) | ~179us | - | baseline |
| scene_render_integrated_clear | - | ~127us | **-29%** |
| rasterize_only (no clear) | ~112us | ~113us | no change |
| clear_only (eliminated) | ~43us | n/a | **-43us saved** |

Combined with ADR-009: **309us -> 127us = 59% cumulative speedup.**

## Consequences

**Positive:**
- 29% faster scene rendering by eliminating cold-cache memset
- Per-tile clearing is effectively free (buffers already L1-hot from rasterization)
- Backward compatible: `clear_color = None` preserves existing behavior
- Works with all three shading paths (flat, gouraud, textured) and both sequential/parallel

**Negative:**
- Adds ~1us overhead to `end_frame` for writing empty tiles
- Code duplication across 6 end_frame loop variants (flat/gouraud/textured x sequential/parallel)
- Empty tiles now always written (even if unchanged) when clear_color is set

**Trade-offs:**
- Worst case (empty scene, all tiles empty): 300 tiles x ~40ns = ~12us vs 43us memset = still faster
- For scenes covering most tiles: overhead is negligible since rasterized tiles already do full clear
