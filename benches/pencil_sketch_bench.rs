use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::pencil_sketch::apply_pencil_sketch;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_pencil_sketch(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    // Fill with some pattern
    let pixels = fb.as_mut_slice();
    for (i, p) in pixels.iter_mut().enumerate() {
        let x = (i % 800) as u8;
        let y = (i / 800) as u8;
        *p = 0xFF000000 | ((x as u32) << 16) | ((y as u32) << 8) | ((x.wrapping_add(y)) as u32);
    }

    c.bench_function("pencil_sketch_800x600", |b| {
        b.iter(|| {
            apply_pencil_sketch(black_box(&mut fb), black_box(5));
        })
    });
}

criterion_group!(benches, bench_pencil_sketch);
criterion_main!(benches);
