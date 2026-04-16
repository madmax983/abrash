use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use abrash_render::experimental::fire::apply_fire;
use abrash_core::framebuffer::Framebuffer;
use std::hint::black_box;
use abrash_core::utils::XorShift32;

fn bench_fire(c: &mut Criterion) {
    let mut group = c.benchmark_group("fire_effect");
    let mut rng = XorShift32::new(1337);

    for (width, height) in [(320, 240), (640, 480), (800, 600)].iter() {
        let mut fb = Framebuffer::new(*width, *height).unwrap();
        let mut cooling_map = vec![0; (*width * *height) as usize];

        // Seed heat
        for y in (*height / 2)..*height {
            for x in 0..*width {
                if rng.next_u32() % 10 == 0 {
                    fb.set_pixel(x as i32, y as i32, 0x00FF0000);
                }
            }
        }

        // Seed cooling map
        for i in 0..cooling_map.len() {
            cooling_map[i] = (rng.next_u32() % 50) as u8;
        }

        group.bench_with_input(BenchmarkId::from_parameter(format!("{}x{}", width, height)), &(*width, *height), |b, _| {
            b.iter(|| black_box(apply_fire(&mut fb, &cooling_map)))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_fire);
criterion_main!(benches);