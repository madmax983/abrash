use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[cfg(feature = "nova")]
use abrash::experimental::sonar::{SonarConfig, apply_sonar};
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;

#[cfg(feature = "nova")]
fn bench_apply_sonar(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let mut zb = ZBuffer::new(1920, 1080).unwrap();

    // Fill with depth and color values
    for y in 0..1080 {
        for x in 0..1920 {
            let r = (x % 256) as u32;
            let g = (y % 256) as u32;
            let b = ((x + y) % 256) as u32;
            fb.set_pixel(x, y, 0xFF00_0000 | (r << 16) | (g << 8) | b);

            // set varied depths
            let depth = 10.0 + (x as f32 / 100.0) + (y as f32 / 100.0);
            zb.test_and_set(x, y, depth);
        }
    }

    let config = SonarConfig::default();

    c.bench_function("apply_sonar_1080p", |b| {
        b.iter(|| {
            apply_sonar(black_box(&mut fb), black_box(&zb), black_box(&config));
        });
    });
}

#[cfg(not(feature = "nova"))]
fn bench_apply_sonar(_c: &mut Criterion) {}

criterion_group!(benches, bench_apply_sonar);
criterion_main!(benches);
