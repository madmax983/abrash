use abrash::experimental::braille::BrailleConverter;
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_braille_converter(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    // Fill with some noise/data so the luminance isn't just constant branches
    for y in 0..600 {
        for x in 0..800 {
            let color = if (x + y) % 2 == 0 {
                0xFF_FFFFFF
            } else {
                0xFF_000000
            };
            fb.set_pixel(x, y, color);
        }
    }

    c.bench_function("braille_converter_800x600", |b| {
        b.iter(|| {
            let converter = BrailleConverter::new(&fb);
            let s = converter.to_string();
            black_box(s);
        });
    });
}

criterion_group!(benches, bench_braille_converter);
criterion_main!(benches);
