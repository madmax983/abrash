use abrash::experimental::pop_art::{PopArtConfig, apply_pop_art};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_pop_art(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut dest = Framebuffer::new(width, height).unwrap();
    let mut source = Framebuffer::new(width, height).unwrap();

    // Fill source with some pattern
    let pixels = source.as_mut_slice();
    for y in 0..height {
        for x in 0..width {
            let color = if (x + y) % 2 == 0 {
                0xFF_FFFFFF
            } else {
                0xFF_000000
            };
            pixels[(y * width + x) as usize] = color;
        }
    }

    let config = PopArtConfig::default();

    c.bench_function("apply_pop_art 1080p", |b| {
        b.iter(|| {
            apply_pop_art(&mut dest, &source, &config);
        });
    });
}

criterion_group!(benches, bench_pop_art);
criterion_main!(benches);
