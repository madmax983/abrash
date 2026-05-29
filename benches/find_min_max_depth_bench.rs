use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::Rng;

// Re-declare the function here for benchmarking or expose it from the library.
// For the sake of the benchmark, we will test the SIMD implementation directly.

#[inline(never)]
fn find_min_max_depth_scalar(depths: &[f32]) -> Option<(f32, f32)> {
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    let mut has_content = false;

    for &z in depths {
        if z != f32::INFINITY {
            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
            has_content = true;
        }
    }

    if has_content {
        Some((min_z, max_z))
    } else {
        None
    }
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
unsafe fn find_min_max_depth_simd(depths: &[f32]) -> Option<(f32, f32)> {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::*;

    let len = depths.len();
    let mut i = 0;

    let mut min_vec = _mm256_set1_ps(f32::MAX);
    let mut max_vec = _mm256_set1_ps(f32::MIN);
    let inf_vec = _mm256_set1_ps(f32::INFINITY);
    let mut has_content = false;

    while i + 8 <= len {
        let depth_val = unsafe { _mm256_loadu_ps(depths.as_ptr().add(i)) };

        // Create a mask where depth != f32::INFINITY
        let is_not_inf = _mm256_cmp_ps(depth_val, inf_vec, _CMP_NEQ_OQ);
        let mask_int = _mm256_movemask_ps(is_not_inf);

        if mask_int != 0 {
            has_content = true;
            // Blend f32::MAX for min where is_inf (so it doesn't affect min)
            let valid_min_vals = _mm256_blendv_ps(_mm256_set1_ps(f32::MAX), depth_val, is_not_inf);
            min_vec = _mm256_min_ps(min_vec, valid_min_vals);

            // Blend f32::MIN for max where is_inf (so it doesn't affect max)
            let valid_max_vals = _mm256_blendv_ps(_mm256_set1_ps(f32::MIN), depth_val, is_not_inf);
            max_vec = _mm256_max_ps(max_vec, valid_max_vals);
        }

        i += 8;
    }

    // Horizontal reduction
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;

    if has_content {
        let mut mins = [0.0f32; 8];
        let mut maxs = [0.0f32; 8];
        unsafe { _mm256_storeu_ps(mins.as_mut_ptr(), min_vec) };
        unsafe { _mm256_storeu_ps(maxs.as_mut_ptr(), max_vec) };

        for j in 0..8 {
            if mins[j] < min_z {
                min_z = mins[j];
            }
            if maxs[j] > max_z {
                max_z = maxs[j];
            }
        }
    }

    // Scalar tail
    for &z in &depths[i..len] {
        if z != f32::INFINITY {
            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
            has_content = true;
        }
    }

    if has_content {
        Some((min_z, max_z))
    } else {
        None
    }
}

fn bench_min_max(c: &mut Criterion) {
    let mut group = c.benchmark_group("Heat Vision Min Max");
    let resolutions = [(320, 240), (800, 600), (1920, 1080)];

    for (w, h) in resolutions {
        let mut rng = rand::thread_rng();
        let depths: Vec<f32> = (0..(w * h)).map(|_| {
            if rng.gen_bool(0.1) {
                f32::INFINITY
            } else {
                rng.gen_range(0.1..100.0)
            }
        }).collect();

        group.bench_function(format!("Scalar {}x{}", w, h), |b| {
            b.iter(|| {
                black_box(find_min_max_depth_scalar(black_box(&depths)));
            });
        });

        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        if std::is_x86_feature_detected!("avx2") {
            group.bench_function(format!("SIMD {}x{}", w, h), |b| {
                b.iter(|| {
                    black_box(unsafe { find_min_max_depth_simd(black_box(&depths)) });
                });
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_min_max);
criterion_main!(benches);
