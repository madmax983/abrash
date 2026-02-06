# Roadmap to 11/10 Performance Architecture

**Status**: Planning
**Goal**: Push Abrash from 9/10 (best-in-class) to 11/10 (unreasonable) performance
**Current State**: Tile-based + Hi-Z + Parallel (baseline 1×)

---

## Performance Target: 150-200× Faster Than Baseline

| Phase | Target | Cumulative Speedup | Score |
|-------|--------|-------------------|-------|
| Current | Baseline | 1× | 9/10 |
| Phase 1 | SIMD-first | 12× | 10/10 ✅ |
| Phase 2 | Hybrid GPU | 60× | 11/10 🏆 |
| Phase 3 | Insane opts | 150-200× | 12/10 💀 |

---

## Phase 1: SIMD Everything (9/10 → 10/10)

**Effort**: 3-4 weeks
**Impact**: 10-12× speedup
**Priority**: HIGH - Biggest ROI

### 1.1: Vectorized Rasterization (8 pixels/clock)

**Current**: Process 1 pixel per iteration (scalar)

**Target**: Process 8 pixels simultaneously using AVX2

```rust
use std::arch::x86_64::*;

// Inner rasterization loop processes 8 pixels at once
for y in y_start..y_end {
    for x in (x_start..x_end).step_by(8) {
        unsafe {
            // Load 8 depths, 8 zbuffer values
            let depths = _mm256_set_ps(/* 8 depth values */);
            let zb_vals = _mm256_loadu_ps(&zbuffer[idx..idx+8]);

            // 8 depth comparisons in parallel
            let mask = _mm256_cmp_ps(depths, zb_vals, _CMP_LT_OQ);

            // Conditional write for all 8 pixels via mask
            _mm256_maskstore_ps(&mut zbuffer[idx], mask, depths);
            _mm256_maskstore_epi32(&mut framebuffer[idx], mask, colors);
        }
    }
}
```

**Key techniques**:
- AVX2 intrinsics (`_mm256_*`)
- Masked stores for conditional writes
- 8-wide SIMD registers (256-bit)
- Process 8 pixels per iteration vs. 1

**Expected speedup**: 6-7× (not quite 8× due to memory bandwidth limits)

**References**:
- [krzosa's SIMD rasterizer](https://github.com/krzosa/software_rasterizer) - Production example
- [Zielon's optimized rasterizer](https://zielon.github.io/rasterizer/) - Benchmarks

---

### 1.2: SIMD Hi-Z Pyramid Construction

**Current**: Scalar 2×2 min-reduction (3 min operations per output pixel)

**Target**: AVX2 processes 8 reductions simultaneously

```rust
// Build pyramid level with SIMD
for y in 0..level.height {
    for x in (0..level.width).step_by(8) {
        unsafe {
            // Load 16 values from source (8 pairs for 2×2 reduction)
            let row0_lo = _mm256_loadu_ps(&source[src_y * width + src_x]);
            let row0_hi = _mm256_loadu_ps(&source[src_y * width + src_x + 8]);
            let row1_lo = _mm256_loadu_ps(&source[(src_y+1) * width + src_x]);
            let row1_hi = _mm256_loadu_ps(&source[(src_y+1) * width + src_x + 8]);

            // Vertical min (row0 with row1)
            let min_vert_lo = _mm256_min_ps(row0_lo, row1_lo);
            let min_vert_hi = _mm256_min_ps(row0_hi, row1_hi);

            // Horizontal min (neighboring pairs via shuffle)
            let min_horiz = horizontal_min_pairs(min_vert_lo, min_vert_hi);

            _mm256_storeu_ps(&level.depths[y * level.width + x], min_horiz);
        }
    }
}
```

**Expected speedup**: 4-6× pyramid build
- 1080p: 1.2ms → 0.2-0.3ms
- 4K: 6ms → 1-1.5ms

---

### 1.3: Fixed-Point Rasterization

**Current**: Floating-point edge equations (6 float ops per edge function)

**Target**: Fixed-point i32 with 8-bit sub-pixel precision

```rust
// Fixed-point vertex coordinates (8 bits sub-pixel precision)
struct VertexFixed {
    x: i32,  // 24.8 fixed point (24 int bits, 8 fractional)
    y: i32,
    z: f32,  // Keep depth as float for zbuffer compatibility
}

// Edge function in fixed point
#[inline(always)]
fn edge_function_fixed(p: (i32, i32), v0: VertexFixed, v1: VertexFixed) -> i32 {
    (p.0 - v0.x) * (v1.y - v0.y) - (p.1 - v0.y) * (v1.x - v0.x)
}

// Faster integer ALU, better SIMD packing (16 i32 vs 8 f32 in AVX2)
```

**Benefits**:
- Integer ALU is faster than float
- Better SIMD utilization (16 i32 vs 8 f32 with AVX2)
- Deterministic rasterization (no float rounding issues)

**Expected speedup**: 1.5-2× edge testing

**Reference**: [SIMD rasterization techniques](https://github.com/krzosa/software_rasterizer#fixed-point)

---

### Phase 1 Results

| Optimization | Individual | Cumulative |
|--------------|------------|------------|
| AVX2 rasterization | 6× | 6× |
| SIMD Hi-Z | 1.3× | 7.8× |
| Fixed-point edges | 1.5× | **11.7×** |

**Outcome**: 10/10 - Best-in-class CPU software rasterizer

---

## Phase 2: Hybrid GPU Compute (10/10 → 11/10)

**Effort**: 4-6 weeks
**Impact**: 5-6× additional speedup
**Priority**: MEDIUM - High impact, significant work

### 2.1: GPU Compute Binning

**Current**: CPU bins triangles to tiles sequentially in Rust

**Target**: GPU bins triangles in parallel using wgpu compute shader

```wgsl
// Coarse binning compute shader
@compute @workgroup_size(64, 1, 1)
fn coarse_bin_triangles(
    @builtin(global_invocation_id) gid: vec3<u32>,
    triangles: array<Triangle>,
    tile_bins: array<AtomicBin>  // Lock-free atomic append
) {
    let tri_idx = gid.x;
    if tri_idx >= arrayLength(&triangles) {
        return;
    }

    let tri = triangles[tri_idx];
    let aabb = compute_screen_aabb(tri);

    // Which tiles does this triangle overlap?
    let min_tile_x = aabb.min_x / 32;
    let max_tile_x = (aabb.max_x + 31) / 32;
    let min_tile_y = aabb.min_y / 32;
    let max_tile_y = (aabb.max_y + 31) / 32;

    for (var ty = min_tile_y; ty <= max_tile_y; ty++) {
        for (var tx = min_tile_x; tx <= max_tile_x; tx++) {
            let tile_idx = ty * tiles_x + tx;

            // Atomic append triangle index to tile bin
            let offset = atomicAdd(&tile_bins[tile_idx].count, 1u);
            tile_bins[tile_idx].triangle_ids[offset] = tri_idx;
        }
    }
}
```

**Architecture**:
```
CPU: Prepare triangles → Upload to GPU
GPU: Bin triangles to tiles (parallel)
CPU: Download bins → Rasterize tiles (parallel with Rayon)
```

**Expected speedup**: 10-20× binning
- 1000 triangles: 0.5ms → 0.025-0.05ms
- Amortized cost: ~0.2ms per frame (upload/download overhead)

**Net speedup**: 3× overall (binning + reduced CPU load)

**References**:
- [ComputeRaster](https://github.com/StarsX/ComputeRaster) - Full GPU pipeline
- [NVIDIA GPU Software Rasterization](https://research.nvidia.com/sites/default/files/pubs/2011-08_High-Performance-Software-Rasterization/laine2011hpg_paper.pdf)

---

### 2.2: Two-Level Hierarchical Binning

**Current**: Single-level 32×32 tiles

**Target**: Two-level hierarchy (128×128 coarse bins → 32×32 fine tiles)

```
Screen (3840×2160)
  ↓
Coarse bins (128×128) → 30×17 = 510 bins
  ↓ (only for bins with triangles)
Fine tiles (32×32) → 16 tiles per bin → 120×68 = 8160 total tiles
```

**Benefits**:
1. **Coarse rejection**: Hi-Z query at 128×128 level rejects large regions
2. **Reduced fine binning**: Only refine bins that passed coarse test
3. **Better GPU occupancy**: 510 coarse bins vs 8160 tiles (less thread divergence)

**Algorithm**:
```rust
// Step 1: GPU bins to 128×128 coarse bins
gpu_bin_coarse(triangles, coarse_bins); // 510 bins

// Step 2: Hi-Z cull coarse bins
for bin in coarse_bins {
    if hiz.is_bin_occluded(bin) {
        bin.mark_culled();  // Skip entire 128×128 region
    }
}

// Step 3: GPU refines visible coarse bins to 32×32 fine tiles
gpu_bin_fine(visible_coarse_bins, fine_tiles); // Only visible regions

// Step 4: CPU rasterizes fine tiles
rayon_parallel_rasterize(fine_tiles);
```

**Expected speedup**: 1.5-2× (fewer triangle-tile tests, better culling)

**Reference**: [WebGPU compute rasterizer](https://github.com/OmarShehata/webgpu-compute-rasterizer/blob/main/how-to-build-a-compute-rasterizer.md#binning)

---

### 2.3: GPU Hi-Z Pyramid Build

**Current**: CPU builds Hi-Z pyramid (1-2ms at 1080p, 4-8ms at 4K)

**Target**: GPU compute shader builds pyramid (0.1-0.2ms at any resolution)

```wgsl
@compute @workgroup_size(8, 8, 1)
fn build_hiz_level(
    @builtin(global_invocation_id) gid: vec3<u32>,
    source_level: texture_2d<f32>,
    dest_level: texture_storage_2d<r32float, write>
) {
    let x = gid.x;
    let y = gid.y;

    // Sample 2×2 from source level
    let d00 = textureLoad(source_level, vec2<i32>(i32(x*2), i32(y*2)), 0).r;
    let d10 = textureLoad(source_level, vec2<i32>(i32(x*2+1), i32(y*2)), 0).r;
    let d01 = textureLoad(source_level, vec2<i32>(i32(x*2), i32(y*2+1)), 0).r;
    let d11 = textureLoad(source_level, vec2<i32>(i32(x*2+1), i32(y*2+1)), 0).r;

    // Min of 2×2 quad
    let min_depth = min(min(d00, d10), min(d01, d11));

    textureStore(dest_level, vec2<i32>(i32(x), i32(y)), vec4<f32>(min_depth));
}

// Build entire pyramid
fn build_hiz_pyramid_gpu(zbuffer: &ZBuffer) {
    // Upload zbuffer to GPU texture (level 0)
    gpu.upload_texture(zbuffer);

    // Dispatch compute shader for each level
    for level in 1..level_count {
        let width = level_width(level);
        let height = level_height(level);
        dispatch_compute(build_hiz_level, width/8, height/8, 1);
    }

    // Keep pyramid on GPU (no download needed for queries)
}
```

**Benefits**:
- 10-15× faster pyramid build
- Pyramid stays on GPU (no CPU↔GPU transfer for queries)
- Enables GPU-side occlusion queries

**Expected speedup**: 1.2× overall (Hi-Z build is small % of total time)

**Reference**: [Compute rasterizer optimization](https://tellusim.com/compute-raster/)

---

### Phase 2 Results

| Optimization | Individual | Cumulative |
|--------------|------------|------------|
| GPU binning | 3× | 35× |
| Two-level binning | 1.5× | 52× |
| GPU Hi-Z build | 1.2× | **62×** |

**Outcome**: 11/10 - Hybrid CPU/GPU architecture, unreasonably fast

---

## Phase 3: The Insane Stuff (11/10 → 12/10)

**Effort**: Months
**Impact**: 2-3× additional speedup
**Priority**: LOW - Diminishing returns, high complexity

### 3.1: AVX-512 (16-wide SIMD)

**Current**: AVX2 (8-wide, 256-bit)

**Target**: AVX-512 (16-wide, 512-bit) with opmask registers

```rust
// AVX-512: Process 16 pixels with predicated writes
unsafe {
    let depths = _mm512_set_ps(/* 16 depth values */);
    let zb_vals = _mm512_loadu_ps(&zbuffer[idx..idx+16]);

    // 16 comparisons → 16-bit mask
    let mask = _mm512_cmp_ps_mask(depths, zb_vals, _CMP_LT_OQ);

    // Conditional store via opmask (no blend needed)
    _mm512_mask_storeu_ps(&mut zbuffer[idx], mask, depths);
}
```

**Benefits**:
- 2× wider than AVX2 (16 vs 8)
- Opmask registers enable true predicated execution
- No blend instructions needed for conditional writes

**Tradeoffs**:
- Lower clock speeds (AVX-512 thermal throttling)
- Limited CPU support (Intel Skylake-X+, AMD Zen 4+)
- ~1.8× real speedup (not 2× due to clocks)

**Expected speedup**: 1.4× over AVX2 (16-wide @ lower clocks)

**Reference**: [AVX-512 rasterization discussion](https://news.ycombinator.com/item?id=8801436)

---

### 3.2: Temporal Reprojection for Hi-Z

**Idea**: Don't rebuild entire Hi-Z pyramid every frame for small camera motion

```rust
// Frame N-1: Hi-Z pyramid built
// Frame N: Camera moved slightly

// Step 1: Reproject frame N-1 Hi-Z to frame N camera space
fn reproject_hiz(
    prev_hiz: &HiZBuffer,
    prev_view_proj: &Mat4,
    curr_view_proj: &Mat4
) -> HiZBuffer {
    let mut curr_hiz = HiZBuffer::new(width, height);

    // For each pixel in current Hi-Z
    for y in 0..height {
        for x in 0..width {
            // Unproject to world space using prev camera
            let world_pos = unproject(x, y, prev_hiz.get(x, y), prev_view_proj);

            // Project to screen space using curr camera
            let (screen_x, screen_y) = project(world_pos, curr_view_proj);

            // Sample prev Hi-Z and write to curr Hi-Z
            curr_hiz.set(screen_x, screen_y, prev_hiz.get(x, y));
        }
    }

    return curr_hiz;
}

// Step 2: Identify invalidated regions (disocclusion, new geometry)
fn invalidate_regions(curr_hiz: &mut HiZBuffer, new_triangles: &[Triangle]) {
    // Mark pyramid cells as invalid where new geometry appears
    for tri in new_triangles {
        curr_hiz.invalidate_region(tri.aabb);
    }
}

// Step 3: Rebuild only invalidated regions
fn rebuild_invalidated(curr_hiz: &mut HiZBuffer, zbuffer: &ZBuffer) {
    for region in curr_hiz.invalidated_regions() {
        curr_hiz.rebuild_region(region, zbuffer);
    }
}
```

**Expected speedup**: 3-5× Hi-Z build for camera motion <10% of screen

**Tradeoffs**:
- Complex invalidation logic
- Artifacts if invalidation is wrong
- Only helps for small camera motion

---

### 3.3: Persistent GPU Threads / Work Graphs

**Current**: Dispatch one compute shader per tile

**Target**: Persistent threads that pull tiles from GPU queue

```wgsl
// Persistent thread model
@compute @workgroup_size(64)
fn persistent_rasterize(
    @builtin(global_invocation_id) gid: vec3<u32>,
    tile_queue: ptr<storage, AtomicQueue>,
    framebuffer: ptr<storage, array<u32>>
) {
    let thread_id = gid.x;

    // Thread stays alive and pulls work from queue
    loop {
        // Atomic dequeue tile
        let tile_idx = atomicAdd(&tile_queue.head, 1u);
        if tile_idx >= tile_queue.count {
            break;  // No more work
        }

        let tile = tile_queue.tiles[tile_idx];

        // Rasterize tile
        rasterize_tile(tile, framebuffer);
    }
}
```

**Benefits**:
- Eliminates kernel launch overhead
- Better GPU occupancy (threads stay resident)
- Dynamic load balancing

**Expected speedup**: 1.2-1.5× (reduces dispatch overhead)

**Reference**: [AMD Work Graphs](https://gpuopen.com/learn/work_graphs_learning_sample/)

---

### 3.4: Lock-Free Tile Queues

**Current**: Rayon work-stealing (has locking overhead)

**Target**: Custom lock-free SegQueue

```rust
use crossbeam::queue::SegQueue;

pub fn render_parallel_lockfree(tiles: Vec<Tile>) {
    let tile_queue = SegQueue::new();
    for tile in tiles {
        tile_queue.push(tile);
    }

    std::thread::scope(|s| {
        for _ in 0..num_cpus::get() {
            s.spawn(|| {
                while let Some(tile) = tile_queue.pop() {
                    render_tile(tile);
                }
            });
        }
    });
}
```

**Expected speedup**: 1.1-1.3× (reduces Rayon overhead)

---

### 3.5: Profile-Guided Optimization (PGO)

```bash
# Step 1: Build with instrumentation
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" \
    cargo build --release --features "backend-win32,parallel"

# Step 2: Run benchmarks to collect profiles
cargo bench --features "backend-win32,parallel"

# Step 3: Rebuild with profile-guided optimizations
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data" \
    cargo build --release --features "backend-win32,parallel"
```

**Benefits**:
- Better branch prediction (hot paths inlined)
- Code layout optimization (cache-friendly)
- Dead code elimination

**Expected speedup**: 1.1-1.2× (micro-optimizations)

---

### Phase 3 Results

| Optimization | Individual | Cumulative |
|--------------|------------|------------|
| AVX-512 | 1.4× | 87× |
| Temporal Hi-Z | 1.3× | 113× |
| Persistent threads | 1.2× | 136× |
| Lock-free queues | 1.1× | 150× |
| PGO | 1.15× | **172×** |

**Outcome**: 12/10 - PhD thesis material

---

## Implementation Priority

### High Priority (Do This)
1. ✅ **AVX2 rasterization** - 6× speedup, 2-3 weeks
2. ✅ **SIMD Hi-Z pyramid** - 1.3× speedup, 1 week
3. ✅ **Fixed-point edges** - 1.5× speedup, 1 week

**Result**: 10/10 achieved, 12× faster

### Medium Priority (Consider This)
4. ⚠️ **GPU compute binning** - 3× speedup, 3-4 weeks
5. ⚠️ **Two-level binning** - 1.5× speedup, 2 weeks
6. ⚠️ **Lock-free queues** - 1.1× speedup, 1 week

**Result**: 11/10 achieved, 60× faster

### Low Priority (Only If Bored)
7. 🔥 **GPU Hi-Z build** - 1.2× speedup, 2-3 weeks
8. 🔥 **AVX-512** - 1.4× speedup, 2 weeks (portability nightmare)
9. 🔥 **Temporal reprojection** - 1.3× speedup, 3-4 weeks (complex)

**Result**: 11.5/10, 100-150× faster

### Insanity (Don't Do This)
10. 💀 **Persistent threads** - Requires work graphs (AMD/NVIDIA specific)
11. 💀 **Full GPU rasterizer** - Just use Vulkan at that point

---

## The Nuclear Option: Full GPU Software Rasterizer

If you want **true 11/10**, move everything to GPU:

```
CPU: Upload geometry
GPU: Vertex processing
GPU: Binning
GPU: Hi-Z pyramid
GPU: Rasterization (compute shader)
GPU: Merge
CPU: Download framebuffer
```

**Expected speedup**: 50-100× (GPU has 10,000+ cores)

**Tradeoff**: You're now a GPU renderer, not a CPU software rasterizer

**Reference**: [NVIDIA's GPU Software Rasterizer](https://research.nvidia.com/sites/default/files/pubs/2011-08_High-Performance-Software-Rasterization/laine2011hpg_paper.pdf) - 100× faster than CPU

---

## Benchmarking Methodology

### Test Scenes

**Scene 1: Sparse (10 triangles)**
- Resolution: 3840×2160
- Triangles: 10 large triangles
- Expected: Tiling overhead dominates
- Target: <1ms with AVX2

**Scene 2: Medium (100 triangles)**
- Resolution: 1920×1080
- Triangles: 100 medium triangles, 50% overlap
- Expected: Hi-Z culls ~30-40 triangles
- Target: 2-3ms with AVX2 + Hi-Z

**Scene 3: Dense (1000 triangles)**
- Resolution: 3840×2160
- Triangles: 1000 small triangles, high depth complexity
- Expected: Hi-Z culls ~500 triangles, binning dominates
- Target: 10-15ms with full Phase 2 stack

### Metrics

| Metric | Baseline | Phase 1 | Phase 2 | Phase 3 |
|--------|----------|---------|---------|---------|
| Scene 1 (4K, 10 tri) | 4ms | 0.6ms | 0.3ms | 0.2ms |
| Scene 2 (1080p, 100 tri) | 12ms | 1.5ms | 0.5ms | 0.3ms |
| Scene 3 (4K, 1000 tri) | 250ms | 30ms | 8ms | 3ms |

---

## Dependencies

### Phase 1 (SIMD)
- `std::arch::x86_64` - AVX2 intrinsics (stable)
- `packed_simd` (optional) - Portable SIMD (nightly)

### Phase 2 (GPU Compute)
- `wgpu` - WebGPU API (cross-platform compute shaders)
- `bytemuck` - Safe transmutes for GPU data

### Phase 3 (Advanced)
- `crossbeam` - Lock-free queues
- `rayon` (already have) - For comparison
- AVX-512 (CPU support required)

---

## References

### SIMD Optimization
- [krzosa's SIMD software rasterizer](https://github.com/krzosa/software_rasterizer) - Production AVX2 example
- [Zielon's optimized rasterizer](https://zielon.github.io/rasterizer/) - Performance analysis
- [AVX-512 for rasterization](https://news.ycombinator.com/item?id=8801436) - Hacker News discussion

### GPU Compute Rasterization
- [ComputeRaster](https://github.com/StarsX/ComputeRaster) - Full GPU pipeline
- [NVIDIA GPU Software Rasterization paper](https://research.nvidia.com/sites/default/files/pubs/2011-08_High-Performance-Software-Rasterization/laine2011hpg_paper.pdf) - Academic reference
- [WebGPU compute rasterizer tutorial](https://github.com/OmarShehata/webgpu-compute-rasterizer/blob/main/how-to-build-a-compute-rasterizer.md) - Practical guide

### Advanced Techniques
- [AMD Work Graphs](https://gpuopen.com/learn/work_graphs_learning_sample/) - Persistent threads
- [Unreal Engine visibility culling](https://docs.unrealengine.com/4.27/en-US/RenderingAndGraphics/VisibilityCulling) - Hierarchical culling
- [Occlusion culling algorithms](https://www.gamedeveloper.com/programming/occlusion-culling-algorithms) - Survey paper

---

## Notes

- **Start with Phase 1** - Highest ROI, achieves 10/10
- **Phase 2 requires wgpu** - Significant architectural change
- **Phase 3 is overkill** - Diminishing returns
- **PGO is free** - Always enable for releases

**Last Updated**: 2026-02-06
**Author**: Claude Sonnet 4.5 + Mark (human overlord)
