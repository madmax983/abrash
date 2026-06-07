use abrash_render::experimental::sharpen::apply_sharpen;
use abrash_render::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_sharpen(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1024, 1024).unwrap();

    for y in 0..1024 {
        for x in 0..1024 {
            let color = if (x + y) % 2 == 0 {
                0x00FF_FFFF
            } else {
                0x0000_0000
            };
            fb.set_pixel(x, y, color);
        }
    }

    c.bench_function("sharpen_1024x1024", |b| {
        b.iter(|| {
            apply_sharpen(
                black_box(&mut fb),
                black_box(&abrash_render::experimental::sharpen::SharpenConfig { amount: 1.0 }),
            );
        });
    });
}

criterion_group!(benches, bench_sharpen);
criterion_main!(benches);
