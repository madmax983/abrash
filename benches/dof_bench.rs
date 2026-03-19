use abrash::framebuffer::Framebuffer;
use abrash::post_process::apply_depth_of_field;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_dof(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Fill with a checkerboard pattern and varied depths
    for y in 0..height {
        for x in 0..width {
            let color = if (x / 50 + y / 50) % 2 == 0 {
                0xFFFF_FFFF
            } else {
                0xFF00_0000
            };
            fb.set_pixel(x as i32, y as i32, color);

            // Create a depth gradient from 0.0 to 10.0
            let depth = (x as f32 / width as f32) * 10.0;
            zb.test_and_set(x as i32, y as i32, depth);
        }
    }

    let config = abrash::post_process::DepthOfFieldConfig {
        focus_dist: 5.0,
        focus_range: 2.0,
        blur_radius: 3,
    };
    c.bench_function("apply_depth_of_field 1080p", |b| {
        b.iter(|| {
            apply_depth_of_field(black_box(&mut fb), black_box(&zb), black_box(&config));
        });
    });
}

criterion_group!(benches, bench_dof);
criterion_main!(benches);
