use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::harmonograph::Harmonograph;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_harmonograph(c: &mut Criterion) {
    let mut group = c.benchmark_group("Harmonograph");

    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut h = Harmonograph::default();
    h.iterations = 10000;

    group.bench_function("harmonograph_render_10k_iters", |b| {
        b.iter(|| {
            fb.clear(0xFF000000);
            h.render(black_box(&mut fb));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_harmonograph);
criterion_main!(benches);
