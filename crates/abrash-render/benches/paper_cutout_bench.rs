use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::paper_cutout::{apply_paper_cutout, PaperCutoutConfig};
use criterion::{Criterion, criterion_group, criterion_main};
use rand::RngExt;
use std::hint::black_box;

fn bench_paper_cutout(c: &mut Criterion) {
    let mut group = c.benchmark_group("Paper Cutout");
    let resolutions = [(320, 240), (800, 600), (1920, 1080)];
    let config = PaperCutoutConfig::default();

    for (w, h) in resolutions {
        let mut fb = Framebuffer::new(w, h).unwrap();
        let mut zb = ZBuffer::new(w, h).unwrap();

        let mut rng = rand::rng();
        for y in 0..h {
            for x in 0..w {
                let depth = if rng.random_bool(0.1) {
                    f32::INFINITY
                } else {
                    rng.random_range(0.1..100.0)
                };
                unsafe {
                    zb.test_and_set_unchecked(x as usize, y as usize, depth);
                }
                fb.set_pixel(x as i32, y as i32, rng.random::<u32>());
            }
        }

        group.bench_function(format!("{w}x{h}"), |b| {
            b.iter(|| {
                apply_paper_cutout(black_box(&mut fb), black_box(&zb), black_box(&config));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_paper_cutout);
criterion_main!(benches);
