use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::string_art::StringArtGenerator;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn string_art_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(200, 200).unwrap();
    // Fill it with a simple circle
    for y in 50..150 {
        for x in 50..150 {
            let dx = x as f32 - 100.0;
            let dy = y as f32 - 100.0;
            if dx * dx + dy * dy < 50.0 * 50.0 {
                fb.set_pixel(x, y, 0xFF00_0000); // Black
            }
        }
    }

    let fb_data = fb.as_slice().to_vec();

    c.bench_function("string_art_generate", |b| {
        b.iter(|| {
            let mut fb_clone = Framebuffer::new(200, 200).unwrap();
            fb_clone.as_mut_slice().copy_from_slice(&fb_data);
            let mut generator = StringArtGenerator::new(black_box(fb_clone), 200, 500);
            black_box(generator.generate());
        })
    });
}

criterion_group!(benches, string_art_benchmark);
criterion_main!(benches);
