use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::stained_glass::apply_stained_glass;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_stained_glass(c: &mut Criterion) {
    let mut group = c.benchmark_group("Stained Glass");

    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFF_FF0000);

    group.bench_function("800x600_cell_20", |b| {
        b.iter(|| {
            apply_stained_glass(
                black_box(&mut fb),
                black_box(20.0),
                black_box(2.0),
                black_box(0xFF_000000),
            );
        });
    });

    let mut fb_large = Framebuffer::new(1920, 1080).unwrap();
    fb_large.clear(0xFF_00FF00);

    group.bench_function("1920x1080_cell_30", |b| {
        b.iter(|| {
            apply_stained_glass(
                black_box(&mut fb_large),
                black_box(30.0),
                black_box(3.0),
                black_box(0xFF_000000),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_stained_glass);
criterion_main!(benches);
