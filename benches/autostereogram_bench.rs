#![cfg(feature = "nova")]

use abrash::experimental::autostereogram::{AutostereogramConfig, apply_autostereogram};
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_autostereogram(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Fill ZBuffer with some varying depth
    for y in 0..height {
        for x in 0..width {
            let depth = ((x as f32 / width as f32) * (y as f32 / height as f32)) * 2.0 - 1.0;
            zb.test_and_set(x as i32, y as i32, depth);
        }
    }

    let config = AutostereogramConfig {
        pattern_width: 100,
        max_shift: 30,
        depth_scale: 1.0,
    };

    c.bench_function("autostereogram_800x600", |b| {
        b.iter(|| {
            apply_autostereogram(black_box(&mut fb), black_box(&zb), black_box(config));
        });
    });
}

criterion_group!(benches, bench_autostereogram);
criterion_main!(benches);
