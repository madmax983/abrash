use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::string_art::apply_string_art;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_string_art(c: &mut Criterion) {
    let mut fb = Framebuffer::new(200, 200).unwrap();
    // Fill with black to make it draw
    fb.clear(0xFF00_0000);

    c.bench_function("string_art_200x200_100pegs_500lines", |b| {
        b.iter(|| {
            apply_string_art(
                black_box(&mut fb),
                black_box(100),
                black_box(500),
                black_box(0.1),
            );
        });
    });
}

criterion_group!(benches, bench_string_art);
criterion_main!(benches);
