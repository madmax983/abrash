# Fixed-Point Edge Walking Implementation Plan

**Date**: 2026-02-06
**Goal**: Convert edge walking to fixed-point arithmetic and re-test SIMD with integers

## Motivation

After SIMD float operations proved 2-4× slower than scalar, we want to test if **integer SIMD** performs better:
- Integer ALU is generally faster than FPU
- Simpler operations (no float edge cases)
- Potentially better masked store performance
- Deterministic rasterization (no floating-point rounding)

## Current State

**Already Implemented:**
- ✅ `VertexFixed` struct (24.8 fixed-point)
- ✅ `edge_function_fixed` (integer edge testing)
- ✅ Tests verifying fixed-point correctness

**Still Using Float:**
- ❌ `EdgeWalker` uses `i64` for x, but `f32` for z
- ❌ Edge interpolation uses floating-point arithmetic
- ❌ SIMD disabled due to poor float performance

## Phase 1: Fixed-Point Edge Walker (Scalar)

### 1.1: Convert EdgeWalker to Fixed-Point

**Current (src/rasterizer.rs):**
```rust
pub struct EdgeWalker {
    pub x: i64,      // 16.16 fixed point
    pub z: f32,      // Float
    dx: i64,
    dz: f32,         // Float
}
```

**Target:**
```rust
pub struct EdgeWalker {
    pub x: i64,      // 16.16 fixed point (unchanged)
    pub z: i32,      // 24.8 fixed point (NEW)
    dx: i64,
    dz: i32,         // 24.8 fixed point (NEW)
}

impl EdgeWalker {
    pub fn new(p0: ScreenPoint, p1: ScreenPoint) -> Self {
        let dx = ((i64::from(p1.x) - i64::from(p0.x)) << 16) / dy;

        // Convert z to 24.8 fixed point
        let z0_fixed = (p0.z * 256.0) as i32;
        let z1_fixed = (p1.z * 256.0) as i32;
        let dz = ((z1_fixed - z0_fixed) as i64) / dy;

        Self {
            x: i64::from(p0.x) << 16,
            z: z0_fixed,
            dx,
            dz: dz as i32,
        }
    }
}
```

### 1.2: Convert Scanline Rasterization

**Current:**
```rust
fn rasterize_scanline_scalar(
    pixels: &mut [u32],
    depths: &mut [f32],    // Float zbuffer
    mut z: f32,            // Float depth
    dz_dx: f32,            // Float gradient
    color: u32,
) {
    for (pixel, depth) in pixels.iter_mut().zip(depths.iter_mut()) {
        if z < *depth {    // Float comparison
            *depth = z;
            *pixel = color;
        }
        z += dz_dx;
    }
}
```

**Target (Option A - Keep ZBuffer Float):**
```rust
fn rasterize_scanline_scalar(
    pixels: &mut [u32],
    depths: &mut [f32],
    mut z_fixed: i32,       // 24.8 fixed point
    dz_dx_fixed: i32,       // 24.8 fixed point
    color: u32,
) {
    for (pixel, depth) in pixels.iter_mut().zip(depths.iter_mut()) {
        let z_float = (z_fixed as f32) / 256.0;  // Convert to float for comparison
        if z_float < *depth {
            *depth = z_float;
            *pixel = color;
        }
        z_fixed += dz_dx_fixed;
    }
}
```

**Target (Option B - Fixed-Point ZBuffer):**
```rust
// Convert entire ZBuffer to i32 (24.8 fixed point)
pub struct ZBuffer {
    depths: Vec<i32>,  // Was Vec<f32>
    // ...
}

fn rasterize_scanline_scalar(
    pixels: &mut [u32],
    depths: &mut [i32],     // Fixed-point zbuffer
    mut z: i32,             // 24.8 fixed point
    dz_dx: i32,             // 24.8 fixed point
    color: u32,
) {
    for (pixel, depth) in pixels.iter_mut().zip(depths.iter_mut()) {
        if z < *depth {     // Integer comparison (fast!)
            *depth = z;
            *pixel = color;
        }
        z += dz_dx;
    }
}
```

**Recommendation**: Start with **Option A** (keep ZBuffer float) for compatibility. If Option B shows significant benefits, we can convert ZBuffer later.

### 1.3: Update Tests

All existing tests should pass with pixel-identical output:
- `single_triangle_renders_identically_to_fill_triangle_3d`
- `two_overlapping_triangles_zbuffer_correctness`
- Fixed-point tests already exist, extend to cover edge walking

### 1.4: Benchmark Scalar Performance

Expected results:
- ✅ **Same or faster**: Integer ALU is generally faster than FPU
- ✅ **Deterministic**: No floating-point rounding issues
- ❌ **Conversion overhead**: Fixed-to-float conversion for zbuffer comparison (Option A)

## Phase 2: Integer SIMD Rasterization

### 2.1: AVX2 with Integer Operations

**Current (float SIMD - DISABLED):**
```rust
let depths_vec = _mm256_set_ps(z + 7*dz, z + 6*dz, ...);
let zb_vals = _mm256_loadu_ps(&depths[i]);
let mask = _mm256_cmp_ps(depths_vec, zb_vals, _CMP_LT_OQ);
_mm256_maskstore_ps(&mut depths[i], mask, depths_vec);
```

**Target (integer SIMD):**
```rust
// Option A: Keep zbuffer float, convert fixed-to-float in SIMD
let z_fixed_vec = _mm256_set_epi32(
    z_fixed + 7*dz_fixed,
    z_fixed + 6*dz_fixed,
    // ... 8 values
);

// Convert i32 to f32 (cvt instruction)
let z_float_vec = _mm256_cvtepi32_ps(z_fixed_vec);
let zb_vals = _mm256_loadu_ps(&depths[i]);
let mask = _mm256_cmp_ps(z_float_vec, zb_vals, _CMP_LT_OQ);
_mm256_maskstore_ps(&mut depths[i], mask, z_float_vec);

// Option B: Integer zbuffer (no conversion needed)
let z_vec = _mm256_set_epi32(z + 7*dz, z + 6*dz, ...);
let zb_vals = _mm256_loadu_si256(&depths[i]);  // Load i32
let mask = _mm256_cmpgt_epi32(zb_vals, z_vec);  // Integer comparison
_mm256_maskstore_epi32(&mut depths[i], mask, z_vec);
```

### 2.2: Potential Benefits

**Integer comparisons:**
- `_mm256_cmpgt_epi32` might be faster than `_mm256_cmp_ps`
- No float denormal handling
- Simpler instruction

**Integer masked stores:**
- `_mm256_maskstore_epi32` vs `_mm256_maskstore_ps`
- **Question**: Does integer mask store have same 10-15 cycle penalty?
- Need to profile to find out!

**Conversion cost (Option A):**
- `_mm256_cvtepi32_ps` adds 3-5 cycles per vector
- Might negate benefits of integer arithmetic
- **Recommendation**: Test both options

### 2.3: Expected Results

**Best case**: Integer SIMD is 2-4× faster than scalar
- Integer ALU + no float overhead
- Masked store penalty might be lower for integers

**Worst case**: Still slower than scalar
- Masked store penalty applies to integers too
- Conversion overhead (Option A) dominates

**Learning**: We'll know if masked stores are the fundamental issue (affects both int and float) or if float operations specifically were the problem.

## Phase 3: Benchmarking

### 3.1: Test Scenarios

1. **Scalar Fixed-Point vs Scalar Float**
   - Measure pure arithmetic improvement
   - Expect 1.1-1.5× speedup

2. **Integer SIMD vs Scalar Fixed-Point**
   - Measure SIMD benefit with integers
   - Hope for 2-4× speedup (vs 2-4× slowdown with float)

3. **Integer SIMD vs Float SIMD**
   - Direct comparison of int vs float SIMD
   - Shows if float was the problem or masked stores

### 3.2: Metrics

| Configuration | Expected Time (µs) | Expected Speedup |
|---------------|-------------------|------------------|
| Scalar Float (baseline) | 457 | 1.0× |
| Scalar Fixed-Point | 350-400 | 1.1-1.3× |
| Float SIMD (previous) | 1840 | 0.25× (4× slower) ❌ |
| Integer SIMD (Option A) | 200-400 | 1.1-2.3× |
| Integer SIMD (Option B) | 100-200 | 2.3-4.6× ✅ |

## Implementation Sequence

### Week 1: Fixed-Point Scalar
1. Convert `EdgeWalker` to fixed-point z
2. Update `rasterize_scanline_scalar` (Option A - keep float zbuffer)
3. All tests pass (pixel-identical)
4. Benchmark scalar performance

### Week 2: Integer SIMD
5. Implement `rasterize_scanline_simd` with integer operations (Option A)
6. Benchmark vs scalar fixed-point
7. If promising, implement Option B (full integer zbuffer)
8. Final benchmarks and decision

### Week 3: Refinement (if beneficial)
9. Optimize conversion overhead
10. Test at different resolutions
11. Document findings

## Success Criteria

**Minimum (Worth Keeping):**
- Scalar fixed-point is ≥1.0× vs scalar float (no regression)
- Integer SIMD is ≥1.5× vs scalar fixed-point
- All tests pass (pixel-identical output)

**Good:**
- Scalar fixed-point is 1.1-1.3× faster
- Integer SIMD is 2-3× faster than scalar
- Overall 2.6-3.9× speedup

**Excellent:**
- Scalar fixed-point is 1.3-1.5× faster
- Integer SIMD is 4-6× faster than scalar
- Overall 5.2-9× speedup

## References

- Current `EdgeWalker`: `src/rasterizer.rs:164`
- Current `VertexFixed`: `src/tile_renderer.rs:94`
- Current `edge_function_fixed`: `src/tile_renderer.rs:152`
- Roadmap: `docs/ROADMAP-TO-11-10.md` Section 1.3

## Notes

- Start with Option A (keep float zbuffer) for compatibility
- Option B (integer zbuffer) is more invasive but potentially faster
- The key question: **Do integer masked stores perform better than float masked stores?**
- If integer SIMD still has 10-15 cycle penalty, this won't help
- But we won't know until we try!

**Last Updated**: 2026-02-06
