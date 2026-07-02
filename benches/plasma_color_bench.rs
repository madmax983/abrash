use abrash_render::experimental::plasma::{plasma_color_modulo, plasma_color_optimized};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_plasma_color(c: &mut Criterion) {
    let mut group = c.benchmark_group("plasma_color");

    group.bench_function("plasma_color_modulo", |b| {
        b.iter(|| {
            for i in 0..100 {
                black_box(plasma_color_modulo(black_box(i as f32 / 100.0)));
            }
        });
    });

    group.bench_function("plasma_color_optimized", |b| {
        b.iter(|| {
            for i in 0..100 {
                black_box(plasma_color_optimized(black_box(i as f32 / 100.0)));
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_plasma_color);
criterion_main!(benches);
