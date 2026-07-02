use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::plasma::apply_plasma;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_plasma(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();

    c.bench_function("apply_plasma_800x600", |b| {
        let mut time = 0.0;
        b.iter(|| {
            apply_plasma(black_box(&mut fb), time, 0.05);
            time += 0.016;
        });
    });
}

criterion_group!(benches, bench_plasma);
criterion_main!(benches);
