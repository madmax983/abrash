use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rand::Rng;

// A scalar baseline implementation for comparison
fn find_min_max_scalar(depths: &[f32]) -> (f32, f32, bool) {
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

    (min_z, max_z, has_content)
}

fn bench_find_min_max(c: &mut Criterion) {
    let mut group = c.benchmark_group("Heat Vision min_z/max_z Pre-Pass");

    let resolutions = [(320, 240), (800, 600), (1920, 1080)];

    for (w, h) in resolutions {
        let mut depths = vec![f32::INFINITY; w * h];
        let mut rng = rand::thread_rng();

        for y in 0..h {
            for x in 0..w {
                if !rng.gen_bool(0.1) {
                    depths[y * w + x] = rng.gen_range(0.1..100.0);
                }
            }
        }

        group.bench_function(format!("Scalar {}x{}", w, h), |b| {
            b.iter(|| {
                black_box(find_min_max_scalar(black_box(&depths)));
            });
        });

        group.bench_function(format!("SIMD {}x{}", w, h), |b| {
            b.iter(|| {
                let min_max = find_min_max_scalar_vs_simd(black_box(&depths));
                black_box(min_max);
            });
        });
    }

    group.finish();
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[target_feature(enable = "avx2")]
unsafe fn find_min_max_simd_isolated(depths: &[f32]) -> (f32, f32, bool) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{
        _CMP_NEQ_OQ, _mm256_blendv_ps, _mm256_cmp_ps, _mm256_loadu_ps, _mm256_max_ps,
        _mm256_min_ps, _mm256_movemask_ps, _mm256_set1_ps, _mm256_storeu_ps,
    };
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{
        _CMP_NEQ_OQ, _mm256_blendv_ps, _mm256_cmp_ps, _mm256_loadu_ps, _mm256_max_ps,
        _mm256_min_ps, _mm256_movemask_ps, _mm256_set1_ps, _mm256_storeu_ps,
    };

    let len = depths.len();
    let mut i = 0;

    let mut min_vec = _mm256_set1_ps(f32::MAX);
    let mut max_vec = _mm256_set1_ps(f32::MIN);
    let inf_vec = _mm256_set1_ps(f32::INFINITY);

    let mut has_content = false;

    while i + 8 <= len {
        unsafe {
            let depth_val = _mm256_loadu_ps(depths.as_ptr().add(i));

            let mask = _mm256_cmp_ps(depth_val, inf_vec, _CMP_NEQ_OQ);

            if _mm256_movemask_ps(mask) != 0 {
                has_content = true;

                min_vec = _mm256_min_ps(min_vec, depth_val);

                let blended_for_max = _mm256_blendv_ps(_mm256_set1_ps(f32::MIN), depth_val, mask);
                max_vec = _mm256_max_ps(max_vec, blended_for_max);
            }
        }
        i += 8;
    }

    let mut min_arr = [f32::MAX; 8];
    let mut max_arr = [f32::MIN; 8];
    unsafe {
        _mm256_storeu_ps(min_arr.as_mut_ptr(), min_vec);
        _mm256_storeu_ps(max_arr.as_mut_ptr(), max_vec);
    }

    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;
    for j in 0..8 {
        if min_arr[j] < min_z {
            min_z = min_arr[j];
        }
        if max_arr[j] > max_z {
            max_z = max_arr[j];
        }
    }

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

    (min_z, max_z, has_content)
}

fn find_min_max_scalar_vs_simd(depths: &[f32]) -> (f32, f32, bool) {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if std::is_x86_feature_detected!("avx2") {
        unsafe {
            return find_min_max_simd_isolated(depths);
        }
    }
    find_min_max_scalar(depths)
}

criterion_group!(benches, bench_find_min_max);
criterion_main!(benches);
