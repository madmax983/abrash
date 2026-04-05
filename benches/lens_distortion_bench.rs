#![cfg(feature = "nova")]

use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::lens_distortion::{apply_lens_distortion, LensDistortionConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn bench_lens_distortion(c: &mut Criterion) {
    let mut group = c.benchmark_group("Lens Distortion Filter");

    for &size in &[256, 512, 1024] {
        let mut fb = Framebuffer::new(size, size).unwrap();
        fb.clear(0xFFFFFFFF);

        let config = LensDistortionConfig {
            distortion: 0.5,
            scale: 0.8,
        };

        group.throughput(criterion::Throughput::Elements((size * size) as u64));
        group.bench_function(format!("{}x{}", size, size), |b| {
            b.iter(|| {
                apply_lens_distortion(black_box(&mut fb), black_box(config));
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_lens_distortion);
criterion_main!(benches);
