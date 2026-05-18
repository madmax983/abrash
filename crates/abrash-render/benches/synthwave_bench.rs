use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::synthwave::apply_synthwave;
use criterion::{Criterion, criterion_group, criterion_main};
use rand::Rng;
use std::hint::black_box;

fn bench_synthwave(c: &mut Criterion) {
    let mut group = c.benchmark_group("Synthwave");

    let resolutions = [(320, 240), (800, 600), (1920, 1080)];

    for (w, h) in resolutions {
        let mut fb = Framebuffer::new(w, h).unwrap();
        let mut zb = ZBuffer::new(w, h).unwrap();

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
                apply_synthwave(black_box(&mut fb), black_box(&zb));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_synthwave);
criterion_main!(benches);
