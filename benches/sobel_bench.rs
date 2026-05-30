use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::sobel::apply_sobel;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_sobel(c: &mut Criterion) {
    let mut group = c.benchmark_group("Sobel Edge Detection");
    let resolutions = [(320, 240), (800, 600)];

    for (w, h) in resolutions {
        let mut fb = Framebuffer::new(w, h).unwrap();
        // Add some noise or lines so the filter has something to do (though it's constant time anyway)
        for y in 0..h {
            for x in 0..w {
                fb.set_pixel(x as i32, y as i32, if (x + y) % 2 == 0 { 0xFF_FFFFFF } else { 0xFF_000000 });
            }
        }

        group.bench_function(format!("{}x{}", w, h), |b| {
            b.iter(|| apply_sobel(black_box(&mut fb)))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_sobel);
criterion_main!(benches);
