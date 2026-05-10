use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::harmonograph::Harmonograph;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_harmonograph(c: &mut Criterion) {
    let mut group = c.benchmark_group("Harmonograph");

    let mut fb = Framebuffer::new(800, 600).unwrap();
    let h = Harmonograph {
        iterations: 10000,
        ..Default::default()
    };

    group.bench_function("harmonograph_render_10k_iters", |b| {
        b.iter(|| {
            fb.clear(0xFF00_0000);
            h.render(black_box(&mut fb));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_harmonograph);
criterion_main!(benches);
