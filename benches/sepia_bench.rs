use abrash_core::framebuffer::Framebuffer;
use abrash_render::post_process::filters::apply_sepia;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn sepia_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    // Fill with some data
    for i in 0..fb.as_slice().len() {
        fb.as_mut_slice()[i] = 0xFF00_0000 | (i as u32);
    }

    c.bench_function("apply_sepia_1080p", |b| {
        b.iter(|| apply_sepia(black_box(&mut fb)));
    });
}

criterion_group!(benches, sepia_benchmark);
criterion_main!(benches);
