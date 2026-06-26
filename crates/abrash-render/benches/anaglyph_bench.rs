use criterion::{black_box, criterion_group, criterion_main, Criterion};

use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::anaglyph::{apply_anaglyph, AnaglyphConfig};

fn anaglyph_benchmark(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let zb = ZBuffer::new(width, height).unwrap();
    let config = AnaglyphConfig::default();

    // Populate frame buffer with dummy values
    let pixels = fb.as_mut_slice();
    for i in 0..(width * height) as usize {
        pixels[i] = 0xFF00_0000 | (i as u32 % 0xFF);
    }

    c.bench_function("apply_anaglyph", |b| {
        b.iter(|| {
            apply_anaglyph(black_box(&mut fb), black_box(&zb), black_box(config));
        })
    });
}

criterion_group!(benches, anaglyph_benchmark);
criterion_main!(benches);
