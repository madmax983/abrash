use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::frosted_glass::apply_frosted_glass;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn frosted_glass_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    // Fill with some data to prevent potential optimizations on empty buffer
    for pixel in fb.as_mut_slice().iter_mut() {
        *pixel = 0xFF_555555;
    }

    c.bench_function("frosted_glass_800x600_r5", |b| {
        b.iter(|| {
            apply_frosted_glass(black_box(&mut fb), black_box(5), black_box(12345));
        });
    });
}

criterion_group!(benches, frosted_glass_benchmark);
criterion_main!(benches);
