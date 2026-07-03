use abrash::experimental::pointillism::{PointillismConfig, apply_pointillism};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_pointillism(c: &mut Criterion) {
    let mut fb = Framebuffer::new(640, 480).unwrap();
    // Fill with a gradient so we don't optimize out uniform colors
    for y in 0..480 {
        for x in 0..640 {
            let r = (x % 255) as u32;
            let g = (y % 255) as u32;
            fb.as_mut_slice()[y * 640 + x] = 0xFF_00_00_00 | (r << 16) | (g << 8);
        }
    }

    let config = PointillismConfig {
        max_radius: 4.0,
        density: 3,
    };

    c.bench_function("pointillism 640x480 r=4 d=3", |b| {
        b.iter(|| apply_pointillism(&fb, config));
    });
}

criterion_group!(benches, bench_pointillism);
criterion_main!(benches);
