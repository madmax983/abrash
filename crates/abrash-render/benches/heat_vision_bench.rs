use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::heat_vision::apply_heat_vision;

fn bench_heat_vision(c: &mut Criterion) {
    let mut group = c.benchmark_group("Heat Vision");
    for (width, height) in [(320, 240), (800, 600), (1920, 1080)].iter() {
        let mut fb = Framebuffer::new(*width, *height).unwrap();
        let mut zb = ZBuffer::new(*width, *height).unwrap();

        let mut depths = zb.as_mut_slice();
        for i in 0..depths.len() {
            depths[i] = (i as f32) / (depths.len() as f32);
        }

        group.bench_with_input(BenchmarkId::from_parameter(format!("{}x{}", width, height)), &(width, height), |b, _| {
            b.iter(|| {
                apply_heat_vision(black_box(&mut fb), black_box(&zb));
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_heat_vision);
criterion_main!(benches);
