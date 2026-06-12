use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::string_art::{StringArtConfig, apply_string_art};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn criterion_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(200, 200).unwrap();
    let config = StringArtConfig {
        num_pins: 100,
        num_lines: 500,
        ..Default::default()
    };

    c.bench_function("string_art", |b| {
        b.iter(|| {
            apply_string_art(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
