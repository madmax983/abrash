use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::sepia::apply_sepia;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench_sepia(c: &mut Criterion) {
    let mut group = c.benchmark_group("Sepia Filter");

    let resolutions = [(320, 240), (800, 600), (1920, 1080)];

    for (w, h) in resolutions {
        let mut fb = Framebuffer::new(w, h).unwrap();

        // Fill with some data to prevent potential optimizations on empty buffers
        for p in fb.as_mut_slice() {
            *p = 0xFF80_8080;
        }

        group.bench_function(format!("{w}x{h}"), |b| {
            b.iter(|| {
                apply_sepia(black_box(&mut fb));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_sepia);
criterion_main!(benches);
