# ADR 011: fill_rect_alpha Loop Invariant Hoisting

**Status:** Accepted
**Date:** 2026-03-27
**Category:** Performance optimization

## Context

`fill_rect_alpha` fills a rectangular region with a single ARGB color, blending
each pixel via `alpha_blend_pixel(color, dst)`. The source color (and therefore
its alpha, pre-multiplied R+B, and pre-multiplied G channels) is constant across
every pixel in the fill.

The inner loop previously called `alpha_blend_pixel()` per pixel, which
re-extracted `alpha = (src >> 24) & 0xFF`, `src_rb * alpha`, and `src_g * alpha`
on every iteration — approximately 4 wasted operations per pixel.

**Hypothesis:** LLVM is not hoisting these invariants through the `#[inline(always)]
const fn` + mutable slice borrow boundary. Manual hoisting should eliminate
redundant work.

## Benchmark Setup

- **Hardware:** Same machine used for all abrash benchmarks
- **Tool:** Criterion 0.5 with 100 samples per size
- **Workload:** Semi-transparent fill (alpha=0x80) on 1024×768 framebuffer
- **Sizes:** 16×16, 32×32, 64×64, 128×128, 256×256

## Baseline (before hoisting)

| Size    | Time       | Throughput   |
|---------|------------|-------------|
| 16×16   | 243.25 ns  | 1.05 Gp/s  |
| 32×32   | 757.16 ns  | 1.35 Gp/s  |
| 64×64   | 2.581 µs   | 1.59 Gp/s  |
| 128×128 | 9.683 µs   | 1.69 Gp/s  |
| 256×256 | 33.607 µs  | 1.95 Gp/s  |

## Change

Hoisted `src_rb * alpha`, `src_g * alpha`, and `inv_alpha` out of the inner
loop. The per-pixel work now only computes destination-dependent terms:

```rust
// Before: called per-pixel
fb_pixels[idx] = alpha_blend_pixel(color, fb_pixels[idx]);

// After: source terms hoisted before loop
let src_rb_a = src_rb * alpha;  // computed once
let src_g_a = src_g * alpha;    // computed once
let inv_alpha = 255 - alpha;    // computed once
// Per-pixel: only dst extraction + blend
let rb = ((src_rb_a + dst_rb * inv_alpha) >> 8) & 0x00FF_00FF;
let g = ((src_g_a + dst_g * inv_alpha) >> 8) & 0x00FF_00FF;
```

## Results (after hoisting)

| Size    | Time       | Throughput   | Speedup |
|---------|------------|-------------|---------|
| 16×16   | 124.48 ns  | 2.06 Gp/s  | **1.96×** |
| 32×32   | 358.48 ns  | 2.86 Gp/s  | **2.12×** |
| 64×64   | 1.221 µs   | 3.35 Gp/s  | **2.11×** |
| 128×128 | 4.648 µs   | 3.52 Gp/s  | **2.08×** |
| 256×256 | 18.168 µs  | 3.61 Gp/s  | **1.85×** |

All improvements statistically significant (p = 0.00).

## Analysis

**LLVM did not hoist the invariants.** Despite `#[inline(always)]` and `const fn`,
the compiler recomputed `src_rb * alpha` and `src_g * alpha` on every pixel.

Likely cause: the mutable `fb_pixels` slice borrow made LLVM conservative about
proving that the source-derived values couldn't alias with destination writes.
The extract-multiply-mask chain through the inlined function boundary gave the
optimizer enough complexity to give up on hoisting.

**Consistent ~2× speedup** across all sizes confirms this was compute-bound waste,
not cache or memory bandwidth related. The slight decrease at 256×256 (1.85× vs
2.12×) suggests we're beginning to approach memory bandwidth limits at larger
fills.

**At 3.6 Gp/s (256×256)**, `fill_rect_alpha` now achieves ~14.4 GB/s effective
bandwidth (read-modify-write, 4 bytes × 2 per pixel), approaching DDR4 limits.

## Decision

**Accept.** Manual hoisting provides a clean, verified 2× speedup with zero
complexity cost. The optimization is local (same function), preserves the API,
and all 34 blitter tests pass.

## Consequences

- `fill_rect_alpha` no longer delegates to `alpha_blend_pixel()` — the blend
  logic is inlined with hoisted constants. This means changes to the blend
  formula must be updated in two places (`alpha_blend_pixel` and
  `fill_rect_alpha`).
- Future optimization: the same hoisting pattern could apply anywhere a constant
  color is blended repeatedly (e.g., UI overlay fills, fog blending).
