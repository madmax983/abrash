use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::radar::{apply_radar, RadarConfig};

fn benchmark_radar(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    let zb = ZBuffer::new(1920, 1080).unwrap();
    let config = RadarConfig {
        center_x: 960.0,
        center_y: 540.0,
        angle: 1.0,
        sweep_width: 0.5,
        grid_color: 0x0000FF00,
        sweep_color: 0x0000AA00,
        blip_color: 0x00FFFFFF,
        bg_color: 0xFF000000,
        ring_spacing: 50.0,
    };

    c.bench_function("apply_radar 1080p", |b| {
        b.iter(|| {
            apply_radar(black_box(&mut fb), black_box(&zb), black_box(&config));
        });
    });
}

criterion_group!(benches, benchmark_radar);
criterion_main!(benches);
