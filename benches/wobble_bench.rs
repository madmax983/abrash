use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::wobble::{apply_wobble, WobbleConfig};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::f32::consts::PI;

fn wobble_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("wobble_effect");

    let resolutions = [(320, 240), (800, 600), (1920, 1080)];

    for &(width, height) in &resolutions {
        let mut fb = Framebuffer::new(width, height).unwrap();

        // Fill with dummy data
        let slice = fb.as_mut_slice();
        for (i, item) in slice.iter_mut().enumerate() {
            *item = 0xFF00_0000 | (i as u32 & 0x00FF_FFFF);
        }

        let config = WobbleConfig {
            amplitude: 10.0,
            frequency: 5.0,
            time: PI / 4.0,
        };

        group.bench_function(format!("wobble_{width}x{height}"), |b| {
            b.iter(|| {
                apply_wobble(black_box(&mut fb), black_box(&config));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, wobble_benchmark);
criterion_main!(benches);
