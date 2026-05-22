#![cfg(feature = "nova")]

use abrash_render::experimental::autostereogram::{AutostereogramConfig, apply_autostereogram};
use abrash_render::framebuffer::Framebuffer;
use abrash_render::zbuffer::ZBuffer;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

fn bench_autostereogram(c: &mut Criterion) {
    let resolutions = [(800, 600), (1920, 1080), (3840, 2160)];
    let mut group = c.benchmark_group("Autostereogram");

    for &(width, height) in &resolutions {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();
        let config = AutostereogramConfig::default();

        // Fill with some data
        fb.clear(0xFFFF_FFFF);
        for y in 0..height {
            for x in 0..width {
                if (x + y) % 2 == 0 {
                    zb.test_and_set(x as i32, y as i32, (x as f32) / (width as f32));
                }
            }
        }

        group.bench_with_input(
            BenchmarkId::new("apply", format!("{}x{}", width, height)),
            &(width, height),
            |b, _| {
                b.iter(|| {
                    apply_autostereogram(&mut fb, &zb, config);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_autostereogram);
criterion_main!(benches);
