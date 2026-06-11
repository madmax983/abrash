use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::string_art::apply_string_art;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_string_art(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    fb.clear(0xFF00_0000); // Black background (high error)

    c.bench_function("string_art 800x600 (200 pins, 500 lines)", |b| {
        b.iter(|| {
            apply_string_art(
                black_box(&mut fb),
                black_box(200),
                black_box(500),
                black_box(0.1),
                black_box(0xFF00_0000),
                black_box(0xFFFF_FFFF),
            );
        });
    });
}

criterion_group!(benches, bench_string_art);
criterion_main!(benches);
