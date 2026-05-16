use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::vignette::{apply_vignette, VignetteConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_vignette(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vignette Filter");

    let resolutions = [(320, 240), (800, 600), (1920, 1080)];
    let config = VignetteConfig::default();

    for (w, h) in resolutions {
        let mut fb = Framebuffer::new(w, h).unwrap();

        group.bench_function(format!("{w}x{h}"), |b| {
            b.iter(|| {
                apply_vignette(black_box(&mut fb), black_box(&config));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_vignette);
criterion_main!(benches);
