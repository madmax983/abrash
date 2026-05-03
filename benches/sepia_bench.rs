use criterion::{criterion_group, criterion_main, Criterion, black_box};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::sepia::apply_sepia;

fn benchmark_sepia(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.as_mut_slice().fill(0xFF808080);

    c.bench_function("sepia_800x600", |b| {
        b.iter(|| {
            apply_sepia(black_box(&mut fb));
        });
    });
}

criterion_group!(benches, benchmark_sepia);
criterion_main!(benches);
