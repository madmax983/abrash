// Micro-benchmarks for SIMD masked store vs blend+store approaches
//
// Tests to answer key questions:
// 1. Are casts truly zero-cost?
// 2. Is integer masked store slower than float masked store?
// 3. Is blend+store faster than masked store?
// 4. What's the best SIMD approach for scanline rasterization?

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use std::arch::x86_64::*;

const BUFFER_SIZE: usize = 4096; // Increased buffer size to reduce loop overhead

/// Test 1: Verify cast operations are zero-cost (register aliasing only)
fn bench_cast_cost(c: &mut Criterion) {
    let mut group = c.benchmark_group("cast_cost");

    let i32_data: Vec<i32> = (0..BUFFER_SIZE as i32).collect();

    group.bench_function("round_trip_cast", |b| {
        b.iter(|| unsafe {
            for chunk in i32_data.chunks_exact(8) {
                let i32_vec = _mm256_loadu_si256(chunk.as_ptr() as *const __m256i);
                let float_vec = _mm256_castsi256_ps(i32_vec);
                let back_to_i32 = _mm256_castps_si256(float_vec);
                black_box(back_to_i32);
            }
        });
    });

    group.bench_function("simd_blend", |b| {
        b.iter_batched(
            || (vec![f32::INFINITY; SMALL_SIZE], vec![0u32; SMALL_SIZE]),
            |(mut depths, mut pixels)| {
                unsafe {
                    run_float_blend_store(&mut depths, &mut pixels, color, SMALL_SIZE);
                }
                black_box((depths, pixels));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("simd_blend", |b| {
        b.iter_batched(
            || (vec![f32::INFINITY; SMALL_SIZE], vec![0u32; SMALL_SIZE]),
            |(mut depths, mut pixels)| {
                unsafe {
                    run_float_blend_store(&mut depths, &mut pixels, color, SMALL_SIZE);
                }
                black_box((depths, pixels));
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Helper to run scalar kernel
fn run_scalar(depths: &mut [f32], pixels: &mut [u32], color: u32) {
    let mut z = 0.5f32;
    let dz = 0.001f32;
    for i in 0..BUFFER_SIZE {
        if z < depths[i] {
            depths[i] = z;
            pixels[i] = color;
        }
        z += dz;
    }
}

/// Helper to run float masked store kernel (matches actual implementation)
#[target_feature(enable = "avx2")]
unsafe fn run_float_maskstore(depths: &mut [f32], pixels: &mut [u32], color: u32, size: usize) {
    let z_start = 0.5f32;
    let dz_dx = 0.001f32;

    let stride_vec = _mm256_set1_ps(8.0 * dz_dx);
    let mut depths_vec = _mm256_set_ps(
        z_start + 7.0 * dz_dx,
        z_start + 6.0 * dz_dx,
        z_start + 5.0 * dz_dx,
        z_start + 4.0 * dz_dx,
        z_start + 3.0 * dz_dx,
        z_start + 2.0 * dz_dx,
        z_start + 1.0 * dz_dx,
        z_start,
    );

    let color_vec = _mm256_set1_epi32(color as i32);

    let mut i = 0;
    while i + 8 <= size {
        let zb_vals = _mm256_loadu_ps(depths.as_ptr().add(i));
        let mask = _mm256_cmp_ps(depths_vec, zb_vals, _CMP_LT_OQ);

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

        depths_vec = _mm256_add_ps(depths_vec, stride_vec);
        i += 8;
    }
}

/// Helper to run float blend store kernel (matches actual implementation)
#[target_feature(enable = "avx2")]
unsafe fn run_float_blend_store(depths: &mut [f32], pixels: &mut [u32], color: u32, size: usize) {
    let z_start = 0.5f32;
    let dz_dx = 0.001f32;

    let stride_vec = _mm256_set1_ps(8.0 * dz_dx);
    let mut depths_vec = _mm256_set_ps(
        z_start + 7.0 * dz_dx,
        z_start + 6.0 * dz_dx,
        z_start + 5.0 * dz_dx,
        z_start + 4.0 * dz_dx,
        z_start + 3.0 * dz_dx,
        z_start + 2.0 * dz_dx,
        z_start + 1.0 * dz_dx,
        z_start,
    );

    let color_vec = _mm256_set1_epi32(color as i32);

    let mut i = 0;
    while i + 8 <= size {
        let zb_vals = _mm256_loadu_ps(depths.as_ptr().add(i));
        let mask = _mm256_cmp_ps(depths_vec, zb_vals, _CMP_LT_OQ);

        // Blend depths
        let blended_depths = _mm256_blendv_ps(zb_vals, depths_vec, mask);
        _mm256_storeu_ps(depths.as_mut_ptr().add(i), blended_depths);

        // Blend pixels
        let pixels_old = _mm256_loadu_si256(pixels.as_ptr().add(i) as *const __m256i);
        let pixels_old_ps = _mm256_castsi256_ps(pixels_old);
        let color_vec_ps = _mm256_castsi256_ps(color_vec);
        let blended_pixels = _mm256_blendv_ps(pixels_old_ps, color_vec_ps, mask);
        _mm256_storeu_si256(
            pixels.as_mut_ptr().add(i) as *mut __m256i,
            _mm256_castps_si256(blended_pixels),
        );

        depths_vec = _mm256_add_ps(depths_vec, stride_vec);
        i += 8;
    }
}

fn bench_always_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("always_update");
    let color = 0xFFFF0000u32;

    group.bench_function("scalar", |b| {
        b.iter_batched(
            || (vec![f32::INFINITY; BUFFER_SIZE], vec![0u32; BUFFER_SIZE]),
            |(mut depths, mut pixels)| {
                run_scalar(&mut depths, &mut pixels, color);
                black_box((depths, pixels));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("simd_maskstore", |b| {
        b.iter_batched(
            || (vec![f32::INFINITY; BUFFER_SIZE], vec![0u32; BUFFER_SIZE]),
            |(mut depths, mut pixels)| {
                unsafe { run_float_maskstore(&mut depths, &mut pixels, color, BUFFER_SIZE) };
                black_box((depths, pixels));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("simd_blend", |b| {
        b.iter_batched(
            || (vec![f32::INFINITY; BUFFER_SIZE], vec![0u32; BUFFER_SIZE]),
            |(mut depths, mut pixels)| {
                unsafe { run_float_blend_store(&mut depths, &mut pixels, color, BUFFER_SIZE) };
                black_box((depths, pixels));
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_never_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("never_update");
    let color = 0xFFFF0000u32;

    group.bench_function("scalar", |b| {
        b.iter_batched(
            || {
                (
                    vec![f32::NEG_INFINITY; BUFFER_SIZE],
                    vec![0u32; BUFFER_SIZE],
                )
            },
            |(mut depths, mut pixels)| {
                run_scalar(&mut depths, &mut pixels, color);
                black_box((depths, pixels));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("simd_maskstore", |b| {
        b.iter_batched(
            || {
                (
                    vec![f32::NEG_INFINITY; BUFFER_SIZE],
                    vec![0u32; BUFFER_SIZE],
                )
            },
            |(mut depths, mut pixels)| {
                unsafe { run_float_maskstore(&mut depths, &mut pixels, color, BUFFER_SIZE) };
                black_box((depths, pixels));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("simd_blend", |b| {
        b.iter_batched(
            || {
                (
                    vec![f32::NEG_INFINITY; BUFFER_SIZE],
                    vec![0u32; BUFFER_SIZE],
                )
            },
            |(mut depths, mut pixels)| {
                unsafe { run_float_blend_store(&mut depths, &mut pixels, color, BUFFER_SIZE) };
                black_box((depths, pixels));
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_small_scanlines(c: &mut Criterion) {
    let mut group = c.benchmark_group("small_scanlines_32px");
    let color = 0xFFFF0000u32;
    // Simulate typical scanline length (32 pixels)
    const SMALL_SIZE: usize = 32;

    group.bench_function("scalar", |b| {
        b.iter_batched(
            || (vec![f32::INFINITY; SMALL_SIZE], vec![0u32; SMALL_SIZE]),
            |(mut depths, mut pixels)| {
                // Inline scalar loop for 32 pixels
                let mut z = 0.5f32;
                let dz = 0.001f32;
                for i in 0..SMALL_SIZE {
                    if z < depths[i] {
                        depths[i] = z;
                        pixels[i] = color;
                    }
                    z += dz;
                }
                black_box((depths, pixels));
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("simd_maskstore", |b| {
        b.iter_batched(
            || (vec![f32::INFINITY; SMALL_SIZE], vec![0u32; SMALL_SIZE]),
            |(mut depths, mut pixels)| {
                unsafe {
                    run_float_maskstore(&mut depths, &mut pixels, color, SMALL_SIZE);
                }
                black_box((depths, pixels));
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cast_cost,
    bench_always_update,
    bench_never_update,
    bench_small_scanlines
);
criterion_main!(benches);
