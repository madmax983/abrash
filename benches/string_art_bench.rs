use abrash::experimental::string_art::{StringArtConfig, apply_string_art};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_string_art(c: &mut Criterion) {
    let width = 512;
    let height = 512;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with a gradient pattern to ensure memory is somewhat dirty/non-uniform
    for y in 0..height {
        for x in 0..width {
            let color = 0xFF00_0000 | ((x & 0xFF) << 16) | ((y & 0xFF) << 8);
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    let config = StringArtConfig {
        num_pegs: 256,
        num_strings: 1000,
        string_alpha: 0.1,
    };

    c.bench_function("apply_string_art 512x512 (1000 strings)", |b| {
        b.iter(|| {
            apply_string_art(black_box(&mut fb), black_box(&config));
        });
    });
}

criterion_group!(benches, benchmark_string_art);
criterion_main!(benches);
