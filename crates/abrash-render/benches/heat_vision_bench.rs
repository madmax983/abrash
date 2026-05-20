use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::heat_vision::apply_heat_vision;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_heat_vision(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Fill with mixed depths
    for i in 0..(width * height) {
        if i % 10 == 0 {
            zb.test_and_set((i % width) as i32, (i / width) as i32, std::f32::INFINITY);
        } else {
            zb.test_and_set((i % width) as i32, (i / width) as i32, (i % 1000) as f32);
        }
    }

    c.bench_function("heat_vision_1080p", |b| {
        b.iter(|| {
            apply_heat_vision(black_box(&mut fb), black_box(&zb));
        })
    });
}

criterion_group!(benches, bench_heat_vision);
criterion_main!(benches);
