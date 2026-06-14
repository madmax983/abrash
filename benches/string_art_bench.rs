#[cfg(feature = "nova")]
use abrash::experimental::string_art::{apply_string_art, StringArtConfig};
#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use criterion::black_box;
use criterion::{Criterion, criterion_group, criterion_main};

#[cfg(feature = "nova")]
fn bench_string_art(c: &mut Criterion) {
    let mut fb = Framebuffer::new(512, 512).unwrap();

    // Fill with a solid black circle on white background
    fb.clear(0xFF_FF_FF_FF);
    abrash::rasterizer::fill_circle(&mut fb, 256, 256, 128, 0xFF_00_00_00);

    let config = StringArtConfig {
        num_pins: 128,
        num_lines: 500,
        ..Default::default()
    };

    let mut group = c.benchmark_group("string_art");
    group.sample_size(10); // Use a smaller sample size as it might be slow

    group.bench_function("512x512_128_pins_500_lines", |b| {
        b.iter(|| {
            apply_string_art(black_box(&mut fb), black_box(&config));
        });
    });

    group.finish();
}

#[cfg(not(feature = "nova"))]
fn bench_string_art(_c: &mut Criterion) {}

criterion_group!(benches, bench_string_art);
criterion_main!(benches);
