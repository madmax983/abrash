use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::sketch::{SketchConfig, apply_sketch};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_sketch(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with some dummy data to simulate a real image
    let pixels = fb.as_mut_slice();
    for (i, p) in pixels.iter_mut().enumerate() {
        let r = (i % 256) as u32;
        let g = ((i / 2) % 256) as u32;
        let b = ((i / 3) % 256) as u32;
        *p = 0xFF00_0000 | (r << 16) | (g << 8) | b;
    }

    let config = SketchConfig::default();

    c.bench_function("apply_sketch_1080p", |b| {
        b.iter(|| {
            apply_sketch(black_box(&mut fb), black_box(&config));
        })
    });
}

criterion_group!(benches, bench_sketch);
criterion_main!(benches);
