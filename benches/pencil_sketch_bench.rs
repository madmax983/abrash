use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::pencil_sketch::{PencilSketchConfig, apply_pencil_sketch};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_pencil_sketch(c: &mut Criterion) {
    let width = 640;
    let height = 480;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient to process different luminances
    for y in 0..height {
        for x in 0..width {
            let val = (x + y) % 256 ;
            let color = 0xFF00_0000 | (val << 16) | (val << 8) | val;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    let config = PencilSketchConfig::default();

    c.bench_function("pencil_sketch_640x480", |b| {
        b.iter(|| {
            apply_pencil_sketch(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_pencil_sketch);
criterion_main!(benches);
