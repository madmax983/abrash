use criterion::{criterion_group, criterion_main, Criterion};
use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::frosted_glass::apply_frosted_glass;

fn bench_frosted_glass(c: &mut Criterion) {
    let mut group = c.benchmark_group("frosted_glass");

    // Test on a typical resolution
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with arbitrary color
    fb.as_mut_slice().fill(0xFF888888);

    group.bench_function("apply_frosted_glass_radius_5", |b| {
        b.iter(|| {
            apply_frosted_glass(&mut fb, 5);
        })
    });

    group.bench_function("apply_frosted_glass_radius_20", |b| {
        b.iter(|| {
            apply_frosted_glass(&mut fb, 20);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_frosted_glass);
criterion_main!(benches);
