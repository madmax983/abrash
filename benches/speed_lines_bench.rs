use abrash::experimental::speed_lines::{SpeedLinesConfig, apply_speed_lines};
use abrash::framebuffer::Framebuffer;
use criterion::{Criterion, criterion_group, criterion_main};

fn criterion_benchmark(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let config = SpeedLinesConfig::default();

    c.bench_function("apply_speed_lines_800x600", |b| {
        b.iter(|| apply_speed_lines(&mut fb, &config));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
