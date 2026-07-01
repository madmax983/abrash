use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rand::Rng;

// A simple scalar equivalent of the min/max pass
fn find_min_max_scalar(depths: &[f32]) -> Option<(f32, f32)> {
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

// Copy the exact SIMD implementation to benchmark it isolated
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
unsafe fn find_min_max_simd_bench(depths: &[f32]) -> Option<(f32, f32)> {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::*;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{
        _CMP_NEQ_OQ, _mm256_blendv_ps, _mm256_cmp_ps, _mm256_loadu_ps, _mm256_max_ps,
        _mm256_min_ps, _mm256_set1_ps, _mm256_storeu_ps,
    };

    let mut min_vec = _mm256_set1_ps(f32::MAX);
    let mut max_vec = _mm256_set1_ps(f32::MIN);
    let inf_vec = _mm256_set1_ps(f32::INFINITY);

    let mut i = 0;
    while i + 8 <= depths.len() {
        let val = _mm256_loadu_ps(depths.as_ptr().add(i));
        let mask = _mm256_cmp_ps(val, inf_vec, _CMP_NEQ_OQ);

        let valid_min = _mm256_blendv_ps(_mm256_set1_ps(f32::MAX), val, mask);
        min_vec = _mm256_min_ps(min_vec, valid_min);

        let valid_max = _mm256_blendv_ps(_mm256_set1_ps(f32::MIN), val, mask);
        max_vec = _mm256_max_ps(max_vec, valid_max);

        i += 8;
    }

    // Horizontal reduction
    let mut min_arr = [0.0f32; 8];
    let mut max_arr = [0.0f32; 8];
    _mm256_storeu_ps(min_arr.as_mut_ptr(), min_vec);
    _mm256_storeu_ps(max_arr.as_mut_ptr(), max_vec);

    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    for &z in &min_arr {
        if z < min_z {
            min_z = z;
        }
    }
    for &z in &max_arr {
        if z > max_z {
            max_z = z;
        }
    }

    // Tail
    for &z in &depths[i..] {
        if z != f32::INFINITY {
            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
        }
    }

    #[allow(clippy::float_cmp)]
    if min_z == f32::MAX && max_z == f32::MIN {
        None
    } else {
        Some((min_z, max_z))
    }
}

fn bench_min_max(c: &mut Criterion) {
    let mut group = c.benchmark_group("Min Max Reduction");

    let resolutions = [(320, 240), (800, 600), (1920, 1080)];

    for (w, h) in resolutions {
        let size = w * h;
        let mut rng = rand::thread_rng();

        let mut depths = vec![0.0f32; size];
        for i in 0..size {
            depths[i] = if rng.gen_bool(0.1) {
                f32::INFINITY
            } else {
                rng.gen_range(0.1..100.0)
            };
        }

        group.bench_function(format!("Scalar/{w}x{h}"), |b| {
            b.iter(|| {
                black_box(find_min_max_scalar(black_box(&depths)));
            });
        });

        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        if std::is_x86_feature_detected!("avx2") {
            group.bench_function(format!("SIMD/{w}x{h}"), |b| {
                b.iter(|| {
                    unsafe { black_box(find_min_max_simd_bench(black_box(&depths))) };
                });
            });
        }
    }

    group.finish();
}

criterion_group!(benches, bench_min_max);
criterion_main!(benches);
