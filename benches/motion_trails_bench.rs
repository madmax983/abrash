use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::motion_trails::{MotionTrailsConfig, apply_motion_trails};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_motion_trails(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let config = MotionTrailsConfig { decay: 0.8 };
    let mut history = Vec::new();

    c.bench_function("apply_motion_trails_1080p", |b| {
        b.iter(|| {
            apply_motion_trails(&mut fb, &mut history, &config);
        });
    });
}

criterion_group!(benches, bench_motion_trails);
criterion_main!(benches);
