use criterion::{criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::hologram::{apply_hologram, HologramConfig};

fn bench_hologram(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill buffer
    for i in 0..(width*height) {
        fb.as_mut_slice()[i as usize] = 0xFF00_0000 | i;
    }

    let config = HologramConfig::default();

    c.bench_function("hologram_effect_800x600", |b| b.iter(|| {
        apply_hologram(&mut fb, &config);
    }));
}

criterion_group!(benches, bench_hologram);
criterion_main!(benches);
