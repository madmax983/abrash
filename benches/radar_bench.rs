#![allow(unused)]
use criterion::{Criterion, black_box, criterion_group, criterion_main};

use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::radar::{RadarConfig, apply_radar};

fn bench_apply_radar(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let zb = ZBuffer::new(800, 600).unwrap();
    let config = RadarConfig::default();

    c.bench_function("apply_radar_800x600", |b| {
        b.iter(|| {
            apply_radar(&mut fb, &zb, black_box(&config));
        });
    });
}

criterion_group!(benches, bench_apply_radar);
criterion_main!(benches);
