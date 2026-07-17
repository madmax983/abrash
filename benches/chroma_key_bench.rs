use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::chroma_key::smooth_chroma_key;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_chroma_key(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fg = Framebuffer::new(width, height).unwrap();
    let mut bg = Framebuffer::new(width, height).unwrap();

    let key_color = 0xFF_00FF00;
    for y in 0..height {
        for x in 0..width {
            if x < width / 2 {
                fg.set_pixel(x as i32, y as i32, key_color);
            } else {
                fg.set_pixel(x as i32, y as i32, 0xFF_FF0000);
            }
            bg.set_pixel(x as i32, y as i32, 0xFF_FFFFFF);
        }
    }

    let mut group = c.benchmark_group("chroma_key");

    group.bench_function("smooth_1080p", |b| {
        b.iter(|| {
            smooth_chroma_key(
                black_box(&mut fg),
                black_box(&bg),
                black_box(key_color),
                black_box(20.0),
                black_box(100.0),
            );
        });
    });
}

criterion_group!(benches, bench_chroma_key);
criterion_main!(benches);
