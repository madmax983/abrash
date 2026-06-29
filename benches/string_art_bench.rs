use abrash::framebuffer::Framebuffer;
use abrash::experimental::string_art::{apply_string_art, StringArtConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_string_art(c: &mut Criterion) {
    let mut fb = Framebuffer::new(256, 256).unwrap();
    for i in 0..fb.width() * fb.height() {
        fb.as_mut_slice()[i as usize] = if i % 2 == 0 { 0xFF_00_00_00 } else { 0xFF_FF_FF_FF };
    }
    let config = StringArtConfig {
        num_pins: 100,
        num_lines: 500,
        ..Default::default()
    };
    c.bench_function("string_art_256x256", |b| {
        b.iter(|| apply_string_art(black_box(&mut fb), black_box(&config)));
    });
}

criterion_group!(benches, bench_string_art);
criterion_main!(benches);
