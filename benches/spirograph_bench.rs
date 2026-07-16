use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::spirograph::Spirograph;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_spirograph(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let s = Spirograph {
        r_fixed: 150.0,
        r_moving: 52.0,
        pen_offset: 75.0,
        inside: true,
        color: 0xFF_FF_00_00,
        resolution: 200,
        rotations: 52, // Complete the closed loop
    };

    c.bench_function("spirograph_render", |b| {
        b.iter(|| {
            s.draw(black_box(&mut fb), 400, 300);
        });
    });
}

criterion_group!(benches, bench_spirograph);
criterion_main!(benches);
