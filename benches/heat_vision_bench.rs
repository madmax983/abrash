use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::heat_vision::apply_heat_vision;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn benchmark_heat_vision(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Populate buffers
    fb.clear(0xFFFF_FFFF);
    // Fill Z buffer with some data (gradient)
    for y in 0..height {
        for x in 0..width {
            let depth = 0.5 + (x as f32 / width as f32) * 0.4;
            zb.test_and_set(x as i32, y as i32, depth);
        }
    }

    c.bench_function("apply_heat_vision 1080p", |b| {
        b.iter(|| {
            apply_heat_vision(black_box(&mut fb), black_box(&zb));
        });
    });
}

criterion_group!(benches, benchmark_heat_vision);
criterion_main!(benches);
