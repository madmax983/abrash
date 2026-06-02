use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::radar::{apply_radar, RadarConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_radar(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = RadarConfig {
        center_x: 400,
        center_y: 300,
        radius: 250,
        angle: 1.0,
        trail_length: std::f32::consts::PI / 2.0,
        color: 0xFF00_FF00,
        grid_color: 0xFF00_4400,
    };

    c.bench_function("apply_radar_800x600", |b| {
        b.iter(|| apply_radar(black_box(&mut fb), black_box(&config)));
    });
}

criterion_group!(benches, bench_radar);
criterion_main!(benches);