use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::heat_vision::apply_heat_vision;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rand::Rng;

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

        group.bench_function(format!("{}x{}", w, h), |b| {
            b.iter(|| {
                apply_heat_vision(black_box(&mut fb), black_box(&zb));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_heat_vision);
criterion_main!(benches);
