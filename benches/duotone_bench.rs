use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::duotone::{apply_duotone, DuotoneConfig};
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_duotone(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();

    // Fill with some gradients to prevent zero-optimization
    for y in 0..height {
        for x in 0..width {
            let r = (x % 255) as u32;
            let g = (y % 255) as u32;
            let b = ((x + y) % 255) as u32;
            fb.set_pixel(x as i32, y as i32, 0xFF000000 | (r << 16) | (g << 8) | b);
        }
    }

    let config = DuotoneConfig::default();

    c.bench_function("duotone_800x600", |b| {
        b.iter(|| {
            apply_duotone(&mut fb, &config);
        });
    });
}

criterion_group!(benches, bench_duotone);
criterion_main!(benches);
