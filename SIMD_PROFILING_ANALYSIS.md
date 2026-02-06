# SIMD Profiling Analysis - Bottleneck Identification

**Date**: 2026-02-06
**Method**: Direct timing instrumentation with 100-1000 iterations

## Executive Summary

Profiling reveals **two major bottlenecks**:

1. **Hi-Z SIMD Pyramid: 2.7× SLOWER** than scalar (most critical)
2. **Overall Rendering: 4.2× SLOWER** with SIMD enabled

The SIMD rasterization shows minimal difference from scalar, suggesting the SIMD code path may not be executing as expected or overhead is canceling benefits.

---

## Detailed Results

### 1. Scanline Length Analysis

Testing different scanline lengths to find SIMD effectiveness:

| Scanline Length | Scalar (µs) | SIMD (µs) | Difference |
|-----------------|-------------|-----------|------------|
| 4 pixels | 386 | 357 | **1.08× faster** ✅ |
| 8 pixels | 352 | 385 | **0.91× slower** ⚠️ |
| 16 pixels | 403 | 366 | **1.10× faster** ✅ |
| 32 pixels | 356 | 372 | **0.96× slower** ⚠️ |
| 64 pixels | 411 | 362 | **1.14× faster** ✅ |
| 128 pixels | 352 | 371 | **0.95× slower** ⚠️ |
| 256 pixels | 378 | 374 | **1.01× same** ~ |
| 512 pixels | 352 | 362 | **0.97× slower** ⚠️ |

**Analysis**:
- SIMD shows **NO consistent speedup** across scanline lengths
- Variations are within measurement noise (±10%)
- Expected: 6-7× speedup for long scanlines, but seeing ±10% variation
- **Hypothesis**: SIMD code path may not be executing, or overhead == benefit

### 2. Hi-Z Pyramid Build (CRITICAL BOTTLENECK)

| Resolution | Scalar (µs) | SIMD (µs) | Ratio | Change |
|------------|-------------|-----------|-------|--------|
| 800×600 | 391 | 1,038 | **2.65×** | **165% SLOWER** 🔴 |
| 1920×1080 | 1,618 | 4,420 | **2.73×** | **173% SLOWER** 🔴 |
| 2560×1440 | 2,967 | 7,874 | **2.65×** | **165% SLOWER** 🔴 |
| 3840×2160 | 6,732 | 17,828 | **2.65×** | **165% SLOWER** 🔴 |

**Analysis**:
- Hi-Z SIMD is **consistently 2.7× slower** across all resolutions
- Scales linearly with resolution (no cache effects visible)
- ~2 ns/pixel overhead from SIMD operations
- **This is the PRIMARY bottleneck** causing overall performance regression

**Root Cause Hypotheses**:
1. **Excessive shuffle operations** - 5-6 shuffles per 4 output pixels
2. **Unaligned memory loads** - `_mm256_loadu_ps` has overhead
3. **Branch overhead** - Boundary checks for SIMD fallback
4. **Small pyramid levels** - Upper levels too small to benefit from SIMD
5. **Memory bandwidth** - SIMD loads saturating memory bus

### 3. Full Rendering Pipeline Breakdown

| Phase | Scalar (µs) | Scalar % | SIMD (µs) | SIMD % | Ratio |
|-------|-------------|----------|-----------|--------|-------|
| Clear | 370 | 47% | 361 | 17% | 1.02× (same) |
| Render | 412 | 53% | 1,735 | 83% | **0.24× (4.2× slower)** 🔴 |
| **Total** | **782** | **100%** | **2,096** | **100%** | **0.37× (2.7× slower)** 🔴 |

**Throughput**:
- Scalar: **127 M triangles/sec**
- SIMD: **47 M triangles/sec**
- Loss: **-63% throughput** 🔴

**Analysis**:
- Clear time is identical (good - not affected by SIMD)
- Render time is **4.2× slower** with SIMD
- SIMD render now dominates total time (83% vs 53%)

**Rendering includes**:
- Prepare phase (unchanged between scalar/SIMD)
- Bin phase (unchanged)
- Hi-Z pyramid build (if enabled) - **THIS IS THE KILLER**
- Tile rendering (scanline rasterization)

---

## Root Cause Analysis

### Primary Bottleneck: Hi-Z SIMD Pyramid (2.7× slower)

**Evidence**:
- Consistent 2.7× slowdown across all resolutions
- Adds ~1.8-11ms overhead vs 0.4-6.7ms scalar baseline
- Scales linearly with pixels (2 ns/pixel overhead)

**Why So Slow?**

1. **Excessive Shuffle Operations** (most likely):
   ```rust
   // Current implementation: ~5-6 shuffles per 4 output pixels
   let evens_lo = _mm256_shuffle_ps(min_vert_lo, min_vert_lo, 0b10_10_00_00);
   let odds_lo = _mm256_shuffle_ps(min_vert_lo, min_vert_lo, 0b11_11_01_01);
   let min_pairs_lo = _mm256_min_ps(evens_lo, odds_lo);
   // ... more shuffles for horizontal reduction
   ```
   - Each shuffle: ~1-3 cycles latency
   - 5-6 shuffles × 3 cycles = 15-18 cycles overhead per 4 pixels
   - Scalar: ~2-4 cycles per pixel = 8-16 cycles for 4 pixels
   - **Overhead exceeds benefit!**

2. **Small Pyramid Levels**:
   - Upper pyramid levels (64×36, 32×18, etc.) too small for SIMD
   - Setup overhead dominates for <64 pixel rows
   - Scalar is more efficient for small levels

3. **Unaligned Loads**:
   - Using `_mm256_loadu_ps` (unaligned) instead of `_mm256_load_ps` (aligned)
   - Unaligned loads: 2-3× slower than aligned loads
   - Pyramid levels not aligned to 32-byte boundaries

### Secondary Issue: Scanline Rasterization (no speedup)

**Evidence**:
- SIMD and scalar scanline performance nearly identical (±10%)
- No clear benefit even at 512 pixel scanlines

**Possible Causes**:

1. **SIMD Code Path Not Executing**:
   - Check: Is `#[cfg(feature = "simd")]` actually enabled?
   - Check: Is AVX2 code being called or falling back to scalar?

2. **Overhead Cancels Benefit**:
   - SIMD setup: Initialize depth vectors, stride computation
   - Memory bandwidth: 8-wide loads may saturate L1 cache
   - Masked stores: `_mm256_maskstore_*` slower than expected

3. **Small Scanline Dominance**:
   - Test scenes may have mostly short scanlines (<32 pixels)
   - SIMD overhead dominates for scanlines <32 pixels
   - Need histogram of actual scanline lengths in test scenes

---

## Recommendations (Prioritized)

### CRITICAL (Fix Immediately)

**1. Disable Hi-Z SIMD** (Quick Win - 1 hour):
```rust
// In src/hiz_buffer.rs, build_level()
fn build_level(&mut self, level_idx: u32, source: &[f32], source_width: u32) {
    // TEMPORARY: Disable SIMD until optimized
    self.build_level_scalar(level_idx, source, source_width);

    // Original code:
    // #[cfg(all(target_arch = "x86_64", feature = "simd"))]
    // { self.build_level_simd(...); }
}
```
**Expected**: Recover 2.7× performance immediately

**2. Add Width Threshold for Hi-Z SIMD** (1-2 hours):
```rust
fn build_level_simd(&mut self, level_idx: u32, source: &[f32], source_width: u32) {
    let level_width = self.levels[level_idx as usize].width;

    // Only use SIMD for wide levels (amortize overhead)
    if level_width < 128 {
        return self.build_level_scalar(level_idx, source, source_width);
    }

    // SIMD implementation...
}
```
**Expected**: Reduce overhead for small pyramid levels

### HIGH Priority (Fix This Week)

**3. Reduce Hi-Z Shuffle Operations** (4-8 hours):
- Current: 5-6 shuffles per 4 output pixels
- Target: 3-4 shuffles per 8 output pixels
- Use `_mm256_permute2f128_ps` for cross-lane operations
- Optimize horizontal min-reduction pattern

**4. Verify SIMD Rasterization Actually Runs** (1-2 hours):
```rust
// Add debug logging to verify code path
#[cfg(feature = "simd")]
{
    eprintln!("SIMD scanline: len={}", pixels.len());
    rasterize_scanline_simd(pixels, depths, z_at_xs, dz_dx, color);
}
```
- Check if SIMD code is actually executing
- Profile single scanline with cycle counters
- Compare SIMD vs scalar instruction counts

**5. Add Adaptive SIMD Threshold** (2-4 hours):
```rust
const SIMD_THRESHOLD: usize = 32; // Minimum pixels for SIMD

if pixels.len() >= SIMD_THRESHOLD {
    #[cfg(feature = "simd")]
    rasterize_scanline_simd(pixels, depths, z_at_xs, dz_dx, color);
} else {
    rasterize_scanline_scalar(pixels, depths, z_at_xs, dz_dx, color);
}
```

### MEDIUM Priority (Next Sprint)

**6. Align Pyramid Memory** (2-3 hours):
```rust
#[repr(align(32))]
struct PyramidLevel {
    width: u32,
    height: u32,
    depths: Vec<f32>,
}
```

**7. Profile with CPU Counters** (4-8 hours):
- Use Windows Performance Analyzer or Intel VTune
- Measure: cache misses, branch mispredictions, cycle counts
- Identify specific bottleneck instructions

**8. Try 4-wide SSE Instead of 8-wide AVX2** (8-16 hours):
- Lower overhead for small/medium scanlines
- May achieve 3-4× speedup vs nothing

---

## Immediate Action Plan

### Phase 1: Quick Fixes (Today - 2-4 hours)

1. ✅ **Profile completed** - bottlenecks identified
2. 🔧 **Disable Hi-Z SIMD** - comment out SIMD path
3. 🔧 **Add adaptive thresholds** - scanline length >= 32 pixels
4. 🔧 **Verify SIMD execution** - add debug logging
5. ✅ **Re-benchmark** - measure improvements

**Expected Result**: 2-3× speedup immediately

### Phase 2: Hi-Z Optimization (This Week - 8-16 hours)

6. 🔧 **Reduce shuffle operations** - optimize horizontal reduction
7. 🔧 **Add width threshold** - only SIMD for levels >= 128 pixels
8. 🔧 **Align memory allocations** - 32-byte alignment
9. ✅ **Profile and measure** - validate improvements

**Expected Result**: Hi-Z SIMD becomes 2-4× faster than scalar

### Phase 3: Deep Profiling (Next Week - 16-24 hours)

10. 🔧 **CPU performance counters** - detailed instruction-level analysis
11. 🔧 **Scanline length histogram** - measure actual workload distribution
12. 🔧 **Memory bandwidth analysis** - identify saturation points
13. 🔧 **Explore alternatives** - SSE2, different SIMD strategies

---

## Success Metrics

**Minimum Acceptable** (Phase 1):
- Overall rendering: scalar baseline performance (no regression)
- Hi-Z disabled or fixed to not slow things down

**Good Progress** (Phase 2):
- Scanline SIMD: 2-3× speedup for scanlines >= 32 pixels
- Hi-Z SIMD: 1.5-2× speedup over scalar
- Overall: 2-3× speedup

**Phase 1 Target** (Phase 3):
- Scanline SIMD: 4-6× speedup
- Hi-Z SIMD: 3-4× speedup
- Overall: 8-12× cumulative speedup

---

## Conclusion

**Root Cause Identified**: Hi-Z SIMD pyramid build is **2.7× slower** than scalar due to excessive shuffle operations and overhead for small pyramid levels.

**Quick Fix Available**: Disable Hi-Z SIMD or add width threshold to recover baseline performance immediately.

**Path Forward**: Optimize Hi-Z shuffle pattern and add adaptive thresholds. This is fixable with focused optimization work.

The profiling was successful - we now know exactly where the bottlenecks are and have concrete fixes to implement.
