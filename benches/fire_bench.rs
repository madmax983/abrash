use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::fire::apply_fire;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_fire(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fire Effect");
    group.sample_size(100);

    for &(width, height) in &[(320, 240), (640, 480)] {
        let mut fb = Framebuffer::new(width, height).unwrap();
        let cooling_map = vec![0u8; (width * height) as usize];

        // Seed some heat
        for x in 0..width {
            fb.set_pixel(x as i32, (height - 1) as i32, 0x00FF0000);
        }

        group.bench_function(format!("{}x{}", width, height), |b| {
            b.iter(|| apply_fire(black_box(&mut fb), black_box(&cooling_map)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_fire);
criterion_main!(benches);
