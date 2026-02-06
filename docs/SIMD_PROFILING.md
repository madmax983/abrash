# SIMD Profiling and Performance Validation

This document describes the CPU cycle-level profiling infrastructure for validating SIMD optimizations and preventing performance regressions.

## Overview

The profiling system provides three levels of performance measurement:

1. **Micro-benchmarks** (`benches/simd_micro.rs`) - Cycle-level profiling of individual operations
2. **Profiling harness** (`benches/simd_profiling.rs`) - Detailed analysis of hot paths
3. **Regression tests** (`tests/simd_regression.rs`) - Automated performance threshold validation

## Tools

### 1. Micro-Benchmarks (`simd_micro.rs`)

Uses RDTSC (Read Time-Stamp Counter) for cycle-accurate measurements.

**Run with:**
```bash
cargo bench --bench simd_micro
```

**Benchmarks:**
- `hiz_reduction_cycles` - Hi-Z 2×2 min reduction performance
- `scanline_rasterization_cycles` - Scanline rendering at different lengths
- `memory_patterns` - Sequential vs random access patterns
- `tile_locality` - Cache locality impact of tile size

**Key Metrics:**
- Cycles per pixel for Hi-Z pyramid build
- Cycles per pixel for scanline rasterization
- Cache hit/miss patterns

### 2. Profiling Harness (`simd_profiling.rs`)

Binary tool for detailed pipeline analysis with both time and cycle measurements.

**Run with:**
```bash
cargo run --release --bin simd_profiling
```

**Analysis:**
1. **Scanline Length Analysis** - Tests SIMD effectiveness at different scanline lengths (4-512 pixels)
2. **Hi-Z Pyramid Profiling** - Measures pyramid build time across resolutions (800×600 to 4K)
3. **Rendering Pipeline Breakdown** - Separates clear time from render time
4. **Cycle-Level Detail** - Reports cycles/pixel for all operations on x86_64

**Output Example:**
```
## 2. Hi-Z Pyramid Build Profiling

1920×1080:   1234 µs/build ( 60 ns/pixel,  3456789 cycles, 1.67 cycles/pixel)
3840×2160:   5234 µs/build ( 62 ns/pixel, 14567890 cycles, 1.75 cycles/pixel)
```

### 3. Regression Tests (`simd_regression.rs`)

Automated tests that fail if SIMD performance degrades below thresholds.

**Run with:**
```bash
cargo test --test simd_regression --release --features simd -- --nocapture
```

**Tests:**
- `test_hiz_pyramid_performance_threshold` - Hi-Z must be <5 cycles/pixel
- `test_scanline_rasterization_performance` - Full pipeline must be <50 cycles/pixel
- `test_hiz_culling_effectiveness` - Hi-Z must provide ≥1.05× speedup
- `test_memory_access_efficiency` - Memory clears must be <2 cycles/pixel

**Thresholds:**
```rust
// Conservative thresholds to prevent regressions
const HIZ_CYCLES_PER_PIXEL_MAX: f64 = 5.0;      // 2-3 is good
const RENDER_CYCLES_PER_PIXEL_MAX: f64 = 50.0;  // Full pipeline
const HIZ_SPEEDUP_MIN: f64 = 1.05;              // 5% minimum improvement
const MEMORY_CYCLES_PER_PIXEL_MAX: f64 = 2.0;   // Sequential writes
```

## Cycle Counting Methodology

### RDTSC Usage

```rust
#[cfg(target_arch = "x86_64")]
#[inline]
fn read_tsc() -> u64 {
    unsafe { std::arch::x86_64::_rdtsc() }
}

fn measure_cycles<F: FnMut()>(mut f: F, iterations: usize) -> u64 {
    // Warmup to stabilize caches
    for _ in 0..100 {
        f();
    }

    // Measure multiple iterations
    let start = read_tsc();
    for _ in 0..iterations {
        f();
    }
    let end = read_tsc();

    (end - start) / iterations as u64
}
```

### Interpretation

**Cycles per pixel** is the key metric:
- **<2 cycles/pixel** - Excellent (memory-bound optimal)
- **2-5 cycles/pixel** - Good (SIMD working well)
- **5-10 cycles/pixel** - Moderate (some overhead)
- **>10 cycles/pixel** - Poor (likely scalar fallback or cache misses)

**Speedup ratios:**
- **≥2.0×** - Excellent SIMD optimization
- **1.2-2.0×** - Good improvement
- **1.05-1.2×** - Marginal (may not be worth complexity)
- **<1.05×** - Ineffective (overhead dominates)

## Usage Guide

### For Development

1. **Before optimizing:**
   ```bash
   cargo run --release --bin simd_profiling > baseline.txt
   ```

2. **After optimizing:**
   ```bash
   cargo run --release --bin simd_profiling > optimized.txt
   diff baseline.txt optimized.txt
   ```

3. **Validate improvement:**
   ```bash
   cargo test --test simd_regression --release --features simd
   ```

### For CI Integration

Add to CI pipeline:
```yaml
- name: SIMD Regression Tests
  run: cargo test --test simd_regression --release --features simd
```

## Expected Performance

### Hi-Z Pyramid Build

| Resolution | Target Cycles/Pixel | Notes |
|------------|---------------------|-------|
| 1920×1080  | 1.5-2.5             | L3 cache fits |
| 3840×2160  | 2.0-3.0             | Memory bound |

### Scanline Rasterization

| Length | SIMD Speedup | Notes |
|--------|--------------|-------|
| 4 px   | 0.8-1.0×     | Setup overhead dominates |
| 8 px   | 1.2-1.5×     | Starting to benefit |
| 16+ px | 1.5-2.0×     | Full SIMD utilization |

### Memory Access

| Pattern    | Cycles/Pixel | Notes |
|------------|--------------|-------|
| Sequential | 0.5-1.0      | Cache-friendly |
| Random     | 10-50        | Cache misses |

## Troubleshooting

### High cycle counts

**Symptoms:** >10 cycles/pixel for simple operations

**Causes:**
- Cache misses (check memory access patterns)
- Excessive shuffles/permutes (check SIMD implementation)
- Scalar fallback (verify SIMD codegen)
- Branch mispredictions (profile with perf)

**Debug:**
```bash
# Check SIMD codegen
cargo rustc --release -- --emit asm

# Profile with perf (Linux)
perf record cargo bench --bench simd_micro
perf report
```

### SIMD slower than scalar

**Symptoms:** Speedup <1.0×

**Causes:**
- Setup overhead (small inputs)
- Unaligned accesses (check data layout)
- Gather/scatter overhead (prefer sequential)
- Excessive masking (check edge cases)

**Fix:**
- Use adaptive thresholds (enable SIMD only for longer scanlines)
- Align data to 16/32-byte boundaries
- Batch operations to amortize setup
- Simplify mask generation

## References

- Intel® 64 and IA-32 Architectures Software Developer's Manual (RDTSC)
- Agner Fog's Optimization Manuals
- [ADR-002: Hierarchical Z-Buffer](adr/002-hierarchical-z-buffer.md)
- Criterion.rs benchmarking guide

## Future Enhancements

- [ ] Add VTune/perf integration for deeper profiling
- [ ] Cache miss counters (via perf events)
- [ ] Branch misprediction tracking
- [ ] SIMD instruction mix analysis
- [ ] Power consumption metrics (RAPL)
