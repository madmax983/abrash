# 2D Blitter Design

**Date**: 2026-03-26
**Status**: Approved
**Author**: Mark + Claude

## Goal

Add a general-purpose, maximum-speed 2D bitmap blitter to abrash. CPU-first with a GPU path available later (same pattern as the 3D rasterizer). This is the core primitive for 2D game rendering: drawing sprites, tiled backgrounds, and UI elements to the framebuffer as fast as the hardware allows.

## API Surface

New module: `crates/abrash-core/src/blitter.rs`

### Types

```rust
/// Source rectangle within a texture atlas.
pub struct SrcRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Result of clipping a blit against framebuffer bounds. Internal type.
struct ClippedBlit {
    src_x: u32,
    src_y: u32,
    dst_x: u32,
    dst_y: u32,
    w: u32,
    h: u32,
}
```

### Functions

Three tiers of blit, each with safe (auto-clipping) and unchecked variants:

| Function | Strategy | Cost per pixel |
|----------|----------|---------------|
| `blit_opaque` / `_unchecked` | `copy_from_slice` (memcpy per row) | ~0 (memory bandwidth) |
| `blit_colorkey` / `_unchecked` | Branch: skip if pixel == key | 1 comparison |
| `blit_alpha` / `_unchecked` | SWAR alpha blend (with 0x00/0xFF fast paths) | ~4 multiplies (worst case) |

Safe variants accept `i32` destination coordinates (supports partially off-screen sprites).
Unchecked variants accept `u32` (caller guarantees in-bounds).

### Clipping

One shared `clip_blit()` function handles all four edges:

- **Left/Top overflow**: `dst < 0` → advance source origin, reduce size
- **Right/Bottom overflow**: `dst + size > fb_size` → reduce size
- **Source validation**: SrcRect clamped to texture dimensions
- Returns `None` if fully off-screen (blit becomes no-op)

Runs once per blit call. The clipped result feeds directly into the unchecked inner loop.

## Inner Loop Strategies

### Opaque Blit

```
for each row:
    dst[dst_offset..dst_offset+w].copy_from_slice(&src[src_offset..src_offset+w])
```

Compiles to `rep movsb` or SIMD memcpy on x86. Hits memory bandwidth ceiling.
64x64 sprite = 16KB total, well within L1.

### Color-Key Blit

```
for each row:
    for each pixel:
        if src_px != key { dst_px = src_px; }
```

Branch prediction friendly — sprites are mostly opaque with transparent edges.

### Alpha Blit

```
for each pixel:
    if alpha == 0xFF { dst = src; }          // fully opaque fast path
    else if alpha == 0x00 { continue; }      // fully transparent fast path
    else { dst = blend_swar(src, dst, alpha); }  // SWAR blend
```

Reuses the `blend_swar` technique from `texture.rs` (~4 multiplies + masks per pixel).
The 0xFF/0x00 early-outs mean most pixels in typical sprites skip blend math.

## Module Structure

```
crates/abrash-core/src/
├── blitter.rs          # SrcRect, ClippedBlit, clip_blit(), all 6 blit functions
├── framebuffer.rs      # existing (unchanged)
├── texture.rs          # existing (unchanged)
└── lib.rs              # add: pub mod blitter;

Re-export: abrash-core::blitter → abrash::blitter
Consumer: use abrash::blitter::{blit_opaque, blit_alpha, SrcRect};
```

## Integration with 3D Rasterizer

The 3D rasterizer in abrash-render can use `blit_opaque_unchecked` for screen-aligned
textured quads (billboards, UI overlays, particle sprites) where perspective correction
is unnecessary. This is a future optimization — no changes to abrash-render initially.

## Testing Strategy

### Correctness
- Blit known pattern → verify exact pixels match manual set_pixel loop
- Opaque blit: row-by-row equality check
- Color-key: verify skipped pixels retain destination color
- Alpha: verify 0x00 (transparent), 0xFF (opaque), 0x80 (50%) against manual blend

### Clipping
- All four edges independently
- All four corners (diagonal clip)
- Fully off-screen (returns without writing)
- Exact fit (no clipping needed)
- Partially visible from each direction

### Edge Cases
- Zero-size SrcRect
- SrcRect exceeding texture bounds
- 1x1 sprite
- Sprite exactly framebuffer-sized
- dst_x/dst_y at i32::MIN

### Benchmarks
Criterion benchmarks for each tier at common sprite sizes:
- 16x16, 32x32, 64x64, 128x128
- Report throughput in megapixels/sec
- Compare opaque blit to raw memcpy baseline

## GPU Path (Future)

Not in initial scope. The CPU blitter establishes the API and baseline metrics.

GPU makes sense when:
- Hundreds of sprites per frame (batch into single draw call with instancing)
- Full-screen alpha compositing (pixel shaders blend for free)
- Scaled/rotated blits (GPU handles at no extra cost)

GPU hurts when:
- Small sprite counts (PCIe upload overhead dominates)
- Simple opaque blits (CPU memcpy already saturates bandwidth)

API-compatible batched interface:
```rust
gpu_blitter.queue(tex, src_rect, dst_x, dst_y, BlitMode::Alpha);
gpu_blitter.flush(&mut fb);  // single dispatch
```

## Estimated Size

~200-300 lines of implementation + ~200 lines of tests + ~50 lines of benchmarks.
