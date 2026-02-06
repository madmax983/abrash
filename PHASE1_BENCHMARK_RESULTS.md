# Phase 1 SIMD Benchmark Results

**Date**: 2026-02-06
**Commit**: (post-Phase 1 implementation)

## Summary

Phase 1 SIMD implementations have been completed:
- ✅ AVX2 vectorized rasterization (8 pixels/clock)
- ✅ SIMD Hi-Z pyramid construction
- ✅ Fixed-point rasterization infrastructure

**Status**: ⚠️ All code compiles and passes tests, but performance results show regressions rather than improvements.

## Benchmark Results

### Scene Rendering (Scalar vs SIMD)

| Scene | Resolution | Triangles | Scalar Time | SIMD Time | Change |
|-------|------------|-----------|-------------|-----------|--------|
| Scene 1 (Sparse) | 4K (3840×2160) | 10 | 7.17 ms | 11.68 ms | **-63% (slower)** |
| Scene 2 (Medium) | 1080p (1920×1080) | 100 | 0.98 ms | 2.33 ms | **-138% (slower)** |
| Scene 3 (Dense) | 4K (3840×2160) | 1000 | 7.15 ms | 11.75 ms | **-64% (slower)** |

### Hi-Z Pyramid Build (Scalar vs SIMD)

| Resolution | Scalar Time | SIMD Time | Change |
|------------|-------------|-----------|--------|
| 1080p | 1.81 ms | 4.43 ms | **-144% (slower)** |
| 4K | 10.26 ms | 18.10 ms | **-76% (slower)** |

### Hi-Z with Scene Rendering

| Scene | Without Hi-Z | With Hi-Z (SIMD) | Change |
|-------|--------------|------------------|--------|
| Scene 2 (1080p, 100 tri) | 2.33 ms | 7.31 ms | **-214% (slower)** |
| Scene 3 (4K, 1000 tri) | 11.75 ms | 30.60 ms | **-160% (slower)** |

## Analysis

### 🔴 Critical Issues Found

1. **SIMD Rasterization Regression**
   - Expected: 6-7× speedup
   - Actual: 1.6× **slower**
   - Possible causes:
     - SIMD overhead exceeds benefit for small scanlines
     - Memory alignment issues
     - Branch misprediction in SIMD path
     - Masked stores may be inefficient

2. **Hi-Z Pyramid Regression**
   - Expected: 4-6× speedup
   - Actual: 2.4× **slower** (1080p), 1.8× **slower** (4K)
   - Possible causes:
     - SIMD shuffle operations inefficient
     - Cache thrashing from SIMD loads/stores
     - Horizontal reduction overhead

3. **Hi-Z Integration Overhead**
   - Hi-Z adds significant overhead even with SIMD
   - Pyramid build cost dominates for small scenes
   - Expected 30-70% culling not materializing in performance gains

### Root Cause Hypotheses

**Hypothesis 1: SIMD Overhead > SIMD Benefit**
- Small scanlines (avg 10-50 pixels) don't amortize SIMD setup cost
- Scalar code benefits from simpler control flow and better branch prediction

**Hypothesis 2: Memory Bandwidth Bottleneck**
- SIMD loads/stores saturate memory bandwidth
- Cache line thrashing from wider memory access patterns
- Scalar code has better cache locality

**Hypothesis 3: Implementation Issues**
- SIMD shuffle operations for horizontal reduction are expensive
- Masked stores (_mm256_maskstore_*) have hidden overhead
- Fixed-point infrastructure created but not integrated (dead code warnings)

## Test Coverage

✅ **All 60 tests passing** with SIMD feature enabled:
- 34 tile_renderer tests
- 16 hiz_buffer tests (including 2 SIMD-specific tests)
- 10 other tests

**Correctness verified**:
- SIMD output is pixel-identical to scalar
- All edge cases handled (clipping, partial tiles, degenerate triangles)
- No regressions in functionality

## Next Steps

### Immediate (Required for Phase 1 Success)

1. **Profile SIMD Code**
   - Use perf/vtune to identify hotspots
   - Measure actual CPU cycles per pixel
   - Check for unexpected stalls or cache misses

2. **Fix Hi-Z SIMD Implementation**
   - Review horizontal reduction algorithm
   - Consider different shuffle strategies
   - May need to fall back to scalar for pyramid build

3. **Optimize SIMD Rasterization**
   - Review masked store usage
   - Consider tile-level SIMD instead of scanline-level
   - Add heuristic to use scalar for short scanlines (<32 pixels)

4. **Integrate Fixed-Point**
   - Currently unused (dead code warnings)
   - Fixed-point edge functions not called from SIMD path
   - Need to connect VertexFixed to actual rasterization

### Medium-Term (Performance Tuning)

5. **Benchmark Methodology**
   - Create more realistic test scenes
   - Vary scanline lengths to find SIMD crossover point
   - Test with different triangle sizes

6. **Alternative SIMD Strategies**
   - Consider Tile-level SIMD (process 8 tiles in parallel) instead of scanline-level
   - Explore block-based rasterization instead of scanline
   - Investigate SIMD for binning phase instead of rasterization

### Long-Term (If SIMD Doesn't Pan Out)

7. **Fallback Plan**
   - Keep infrastructure but disable by default
   - Focus on parallel (Rayon) for speedup instead
   - Consider Phase 2 (GPU compute) as primary performance path

## Conclusion

**Phase 1 Status: ⚠️ IMPLEMENTED BUT NOT PERFORMANT**

All Phase 1 SIMD optimizations have been implemented and are functionally correct (all tests pass, output is pixel-identical). However, performance benchmarks show **significant regressions** rather than the expected 12× speedup.

**Key Takeaway**: SIMD is tricky. The infrastructure is sound, but tuning is required.

**Recommendation**: Before proceeding to Phase 2, invest 1-2 weeks in profiling and optimization to understand why SIMD is underperforming. If SIMD cannot be made fast, consider alternative strategies (better parallelism, different SIMD approach, or skip to Phase 2 GPU compute).

---

**Benchmarking Environment**:
- CPU: x86_64 with AVX2 support
- OS: Windows
- Rust: 1.x (2024 edition)
- Build: `--release` (optimized)
- Features: `backend-win32,simd`
