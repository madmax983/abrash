use abrash_core::framebuffer::Framebuffer;
use abrash_render::rasterizer::{draw_line_2d, draw_thick_line_2d};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_draw_line_2d(c: &mut Criterion) {
    let mut group = c.benchmark_group("draw_line_2d");
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let color = 0xFFFF_FFFF;

    group.bench_function("horizontal_800px", |b| {
        b.iter(|| {
            draw_line_2d(black_box(&mut fb), 0, 300, 799, 300, black_box(color));
        });
    });

    group.bench_function("vertical_600px", |b| {
        b.iter(|| {
            draw_line_2d(black_box(&mut fb), 400, 0, 400, 599, black_box(color));
        });
    });

    group.bench_function("diagonal_800px", |b| {
        b.iter(|| {
            draw_line_2d(black_box(&mut fb), 0, 0, 799, 599, black_box(color));
        });
    });

    group.bench_function("thick_diagonal_5px", |b| {
        b.iter(|| {
            draw_thick_line_2d(
                black_box(&mut fb),
                0,
                0,
                799,
                599,
                black_box(5),
                black_box(color),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_draw_line_2d);
criterion_main!(benches);
