use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::posterize::{PosterizeConfig, apply_posterize};
use criterion::{Criterion, criterion_group, criterion_main};
use rand::Rng;
use std::hint::black_box;

fn bench_posterize(c: &mut Criterion) {
    let mut group = c.benchmark_group("Posterize Effect");

    let resolutions = [(320, 240), (800, 600), (1920, 1080)];

    let config = PosterizeConfig { levels: 4.0 };

    for (w, h) in resolutions {
        let mut fb = Framebuffer::new(w, h).unwrap();

        // Fill with random noise
        let mut rng = rand::thread_rng();
        for p in fb.as_mut_slice().iter_mut() {
            *p = rng.r#gen::<u32>() | 0xFF00_0000;
        }

        group.bench_function(format!("{w}x{h}"), |b| {
            b.iter(|| {
                apply_posterize(black_box(&mut fb), black_box(&config));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_posterize);
criterion_main!(benches);
