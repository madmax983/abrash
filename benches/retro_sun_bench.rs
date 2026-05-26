use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::retro_sun::render_retro_sun;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_retro_sun(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    let mut group = c.benchmark_group("retro_sun");
    group.bench_function("render_retro_sun_800x600", |b| {
        b.iter(|| {
            fb.clear(0xFF_000000);
            render_retro_sun(
                black_box(&mut fb),
                black_box(400),
                black_box(300),
                black_box(250),
                black_box(0xFF_FF0055),
                black_box(0xFF_FFAA00),
            );
        })
    });
    group.finish();
}

criterion_group!(benches, bench_retro_sun);
criterion_main!(benches);
