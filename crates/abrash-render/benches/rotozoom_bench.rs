use abrash_core::framebuffer::Framebuffer;
use abrash_core::texture::Texture;
use abrash_render::experimental::rotozoom::render_rotozoom;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_rotozoom(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let tex = Texture::new(256, 256).unwrap();

    let mut group = c.benchmark_group("rotozoom");

    group.bench_function("rotozoom_800x600", |b| {
        b.iter(|| {
            render_rotozoom(
                black_box(&mut fb),
                black_box(&tex),
                black_box(45.0),
                black_box(1.5),
                black_box(128.0),
                black_box(128.0),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_rotozoom);
criterion_main!(benches);
