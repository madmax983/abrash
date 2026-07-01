use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::heat_vision::apply_heat_vision;
use criterion::{Criterion, criterion_group, criterion_main};
use rand::Rng;
use std::hint::black_box;

fn bench_heat_vision(c: &mut Criterion) {
    let mut group = c.benchmark_group("Heat Vision");

    let resolutions = [(320, 240), (800, 600), (1920, 1080)];

    for (w, h) in resolutions {
        let mut fb = Framebuffer::new(w, h).unwrap();
        let mut zb = ZBuffer::new(w, h).unwrap();

        // Fill Z-buffer with random depths between 0.1 and 100.0, plus some infinity
        let mut rng = rand::thread_rng();
        for y in 0..h {
            for x in 0..w {
                let depth = if rng.gen_bool(0.1) {
                    f32::INFINITY
                } else {
                    rng.gen_range(0.1..100.0)
                };
                unsafe {
                    zb.test_and_set_unchecked(x as usize, y as usize, depth);
                }
            }
        }

        group.bench_function(format!("{w}x{h}"), |b| {
            b.iter(|| {
                apply_heat_vision(black_box(&mut fb), black_box(&zb));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_heat_vision);
criterion_main!(benches, min_max_benches);

fn bench_find_min_max_depth_simd(c: &mut Criterion) {
    let mut group = c.benchmark_group("Heat Vision Min Max");
    let resolutions = [(320, 240), (800, 600), (1920, 1080)];

    for (w, h) in resolutions {
        let mut depths = vec![0.0; w * h];
        let mut rng = rand::thread_rng();
        for d in depths.iter_mut() {
            *d = if rng.gen_bool(0.1) {
                f32::INFINITY
            } else {
                rng.gen_range(0.1..100.0)
            };
        }

        group.bench_function(format!("SIMD {w}x{h}"), |b| {
            b.iter(|| {
                #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
                if std::is_x86_feature_detected!("avx2") {
                    unsafe {
                        let result = abrash_render::heat_vision::find_min_max_depth_simd(&depths);
                        black_box(result);
                    }
                }
            });
        });

        group.bench_function(format!("Scalar {w}x{h}"), |b| {
            b.iter(|| {
                let mut min_z = f32::MAX;
                let mut max_z = f32::MIN;
                let mut has_content = false;
                for &z in depths.iter() {
                    if z != f32::INFINITY {
                        if z < min_z { min_z = z; }
                        if z > max_z { max_z = z; }
                        has_content = true;
                    }
                }
                black_box((min_z, max_z, has_content));
            });
        });
    }

    group.finish();
}

criterion_group!(min_max_benches, bench_find_min_max_depth_simd);
