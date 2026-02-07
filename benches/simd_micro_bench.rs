// Micro-benchmarks for SIMD masked store vs blend+store approaches
//
// Tests to answer key questions:
// 1. Are casts truly zero-cost?
// 2. Is integer masked store slower than float masked store?
// 3. Is blend+store faster than masked store?
// 4. What's the best SIMD approach for scanline rasterization?

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::arch::x86_64::*;

const BUFFER_SIZE: usize = 1024;

/// Test 1: Verify cast operations are zero-cost (register aliasing only)
fn bench_cast_cost(c: &mut Criterion) {
    let mut group = c.benchmark_group("cast_cost");

    let i32_data: Vec<i32> = (0..BUFFER_SIZE as i32).collect();

    group.bench_function("round_trip_cast", |b| {
        b.iter(|| {
            unsafe {
                for chunk in i32_data.chunks_exact(8) {
                    let i32_vec = _mm256_loadu_si256(chunk.as_ptr() as *const __m256i);

                    // Cast i32 → f32 (should be free)
                    let float_vec = _mm256_castsi256_ps(i32_vec);

                    // Cast f32 → i32 (should be free)
                    let back_to_i32 = _mm256_castps_si256(float_vec);

                    // Force compiler not to optimize away
                    black_box(back_to_i32);
                }
            }
        });
    });

    group.finish();
}

/// Test 2: Scalar conditional write (baseline)
fn bench_scalar_conditional(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalar_conditional");

    let mut depths = vec![f32::INFINITY; BUFFER_SIZE];
    let mut pixels = vec![0u32; BUFFER_SIZE];
    let color = 0xFFFF0000u32;

    group.bench_function("scalar_write", |b| {
        b.iter(|| {
            let mut z = 0.5f32;
            let dz = 0.001f32;

            for i in 0..BUFFER_SIZE {
                if z < depths[i] {
                    depths[i] = z;
                    pixels[i] = color;
                }
                z += dz;
            }

            black_box(&pixels);
            black_box(&depths);
        });
    });

    group.finish();
}

/// Test 3: Integer masked store
fn bench_integer_masked_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("integer_masked_store");

    let mut depths = vec![f32::INFINITY; BUFFER_SIZE];
    let mut pixels = vec![0u32; BUFFER_SIZE];
    let color = 0xFFFF0000u32;

    group.bench_function("i32_maskstore", |b| {
        b.iter(|| {
            unsafe {
                let mut z_fixed = (0.5f32 * 256.0) as i32;
                let dz_fixed = (0.001f32 * 256.0) as i32;
                const INV_256: f32 = 1.0 / 256.0;

                let stride = dz_fixed * 8;
                let color_vec = _mm256_set1_epi32(color as i32);

                let mut i = 0;
                while i + 8 <= BUFFER_SIZE {
                    // Setup depth vector (i32)
                    let depths_vec = _mm256_set_epi32(
                        z_fixed + 7 * dz_fixed,
                        z_fixed + 6 * dz_fixed,
                        z_fixed + 5 * dz_fixed,
                        z_fixed + 4 * dz_fixed,
                        z_fixed + 3 * dz_fixed,
                        z_fixed + 2 * dz_fixed,
                        z_fixed + 1 * dz_fixed,
                        z_fixed,
                    );

                    // Convert to float for comparison
                    let z_float = _mm256_cvtepi32_ps(depths_vec);
                    let z_scaled = _mm256_mul_ps(z_float, _mm256_set1_ps(INV_256));

                    // Load zbuffer
                    let zb_vals = _mm256_loadu_ps(depths.as_ptr().add(i));

                    // Compare
                    let mask = _mm256_cmp_ps(z_scaled, zb_vals, _CMP_LT_OQ);

                    // Masked store (integer version)
                    _mm256_maskstore_ps(
                        depths.as_mut_ptr().add(i),
                        _mm256_castps_si256(mask),
                        z_scaled,
                    );
                    _mm256_maskstore_epi32(
                        pixels.as_mut_ptr().add(i) as *mut i32,
                        _mm256_castps_si256(mask),
                        color_vec,
                    );

                    z_fixed += stride;
                    i += 8;
                }
            }

            black_box(&pixels);
            black_box(&depths);
        });
    });

    group.finish();
}

/// Test 4: Float masked store (for comparison)
fn bench_float_masked_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("float_masked_store");

    let mut depths = vec![f32::INFINITY; BUFFER_SIZE];
    let mut pixels = vec![0u32; BUFFER_SIZE];
    let color = 0xFFFF0000u32;

    group.bench_function("f32_maskstore", |b| {
        b.iter(|| {
            unsafe {
                let mut z = 0.5f32;
                let dz = 0.001f32;

                let stride = 8.0 * dz;
                let color_vec = _mm256_set1_epi32(color as i32);

                let mut i = 0;
                while i + 8 <= BUFFER_SIZE {
                    // Setup depth vector (f32)
                    let depths_vec = _mm256_set_ps(
                        z + 7.0 * dz,
                        z + 6.0 * dz,
                        z + 5.0 * dz,
                        z + 4.0 * dz,
                        z + 3.0 * dz,
                        z + 2.0 * dz,
                        z + 1.0 * dz,
                        z,
                    );

                    // Load zbuffer
                    let zb_vals = _mm256_loadu_ps(depths.as_ptr().add(i));

                    // Compare
                    let mask = _mm256_cmp_ps(depths_vec, zb_vals, _CMP_LT_OQ);

                    // Masked store (float version)
                    _mm256_maskstore_ps(
                        depths.as_mut_ptr().add(i),
                        _mm256_castps_si256(mask),
                        depths_vec,
                    );
                    _mm256_maskstore_epi32(
                        pixels.as_mut_ptr().add(i) as *mut i32,
                        _mm256_castps_si256(mask),
                        color_vec,
                    );

                    z += stride;
                    i += 8;
                }
            }

            black_box(&pixels);
            black_box(&depths);
        });
    });

    group.finish();
}

/// Test 5: Float blend + unconditional store
fn bench_float_blend_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("float_blend_store");

    let mut depths = vec![f32::INFINITY; BUFFER_SIZE];
    let mut pixels = vec![0u32; BUFFER_SIZE];
    let color = 0xFFFF0000u32;

    group.bench_function("f32_blend", |b| {
        b.iter(|| {
            unsafe {
                let mut z = 0.5f32;
                let dz = 0.001f32;

                let stride = 8.0 * dz;
                let color_vec = _mm256_set1_epi32(color as i32);

                let mut i = 0;
                while i + 8 <= BUFFER_SIZE {
                    // Setup depth vector (f32)
                    let depths_vec = _mm256_set_ps(
                        z + 7.0 * dz,
                        z + 6.0 * dz,
                        z + 5.0 * dz,
                        z + 4.0 * dz,
                        z + 3.0 * dz,
                        z + 2.0 * dz,
                        z + 1.0 * dz,
                        z,
                    );

                    // Load zbuffer
                    let zb_vals = _mm256_loadu_ps(depths.as_ptr().add(i));

                    // Compare
                    let mask = _mm256_cmp_ps(depths_vec, zb_vals, _CMP_LT_OQ);

                    // Blend + unconditional store (NEW APPROACH)
                    let blended_depths = _mm256_blendv_ps(zb_vals, depths_vec, mask);
                    _mm256_storeu_ps(depths.as_mut_ptr().add(i), blended_depths);

                    let pixels_old = _mm256_loadu_si256(pixels.as_ptr().add(i) as *const __m256i);
                    let pixels_old_ps = _mm256_castsi256_ps(pixels_old);
                    let color_vec_ps = _mm256_castsi256_ps(color_vec);
                    let blended_pixels = _mm256_blendv_ps(pixels_old_ps, color_vec_ps, mask);
                    _mm256_storeu_si256(
                        pixels.as_mut_ptr().add(i) as *mut __m256i,
                        _mm256_castps_si256(blended_pixels),
                    );

                    z += stride;
                    i += 8;
                }
            }

            black_box(&pixels);
            black_box(&depths);
        });
    });

    group.finish();
}

/// Test 6: Integer blend + unconditional store (BEST APPROACH)
fn bench_integer_blend_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("integer_blend_store");

    let mut depths = vec![f32::INFINITY; BUFFER_SIZE];
    let mut pixels = vec![0u32; BUFFER_SIZE];
    let color = 0xFFFF0000u32;

    group.bench_function("i32_blend", |b| {
        b.iter(|| {
            unsafe {
                let mut z_fixed = (0.5f32 * 256.0) as i32;
                let dz_fixed = (0.001f32 * 256.0) as i32;
                const INV_256: f32 = 1.0 / 256.0;

                let stride = dz_fixed * 8;
                let color_vec = _mm256_set1_epi32(color as i32);

                let mut i = 0;
                while i + 8 <= BUFFER_SIZE {
                    // Setup depth vector (i32)
                    let depths_vec = _mm256_set_epi32(
                        z_fixed + 7 * dz_fixed,
                        z_fixed + 6 * dz_fixed,
                        z_fixed + 5 * dz_fixed,
                        z_fixed + 4 * dz_fixed,
                        z_fixed + 3 * dz_fixed,
                        z_fixed + 2 * dz_fixed,
                        z_fixed + 1 * dz_fixed,
                        z_fixed,
                    );

                    // Convert to float for comparison
                    let z_float = _mm256_cvtepi32_ps(depths_vec);
                    let z_scaled = _mm256_mul_ps(z_float, _mm256_set1_ps(INV_256));

                    // Load zbuffer
                    let zb_vals = _mm256_loadu_ps(depths.as_ptr().add(i));

                    // Compare
                    let mask = _mm256_cmp_ps(z_scaled, zb_vals, _CMP_LT_OQ);

                    // Blend + unconditional store (NEW APPROACH)
                    let blended_depths = _mm256_blendv_ps(zb_vals, z_scaled, mask);
                    _mm256_storeu_ps(depths.as_mut_ptr().add(i), blended_depths);

                    let pixels_old = _mm256_loadu_si256(pixels.as_ptr().add(i) as *const __m256i);
                    let pixels_old_ps = _mm256_castsi256_ps(pixels_old);
                    let color_vec_ps = _mm256_castsi256_ps(color_vec);
                    let blended_pixels = _mm256_blendv_ps(pixels_old_ps, color_vec_ps, mask);
                    _mm256_storeu_si256(
                        pixels.as_mut_ptr().add(i) as *mut __m256i,
                        _mm256_castps_si256(blended_pixels),
                    );

                    z_fixed += stride;
                    i += 8;
                }
            }

            black_box(&pixels);
            black_box(&depths);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cast_cost,
    bench_scalar_conditional,
    bench_integer_masked_store,
    bench_float_masked_store,
    bench_float_blend_store,
    bench_integer_blend_store
);
criterion_main!(benches);
