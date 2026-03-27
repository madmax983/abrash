use abrash::framebuffer::Framebuffer;
use abrash_render::experimental::wobble::{apply_wobble, WobbleConfig};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_wobble(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();

    for y in 0..height {
        for x in 0..width {
            fb.set_pixel(x as i32, y as i32, (x * y) as u32);
        }
    }

    let config = WobbleConfig {
        amplitude: 20.0,
        frequency: 5.0,
        time: 1.0,
    };

    let mut group = c.benchmark_group("wobble");
    group.bench_function("apply_wobble_1080p", |b| {
        b.iter(|| {
            apply_wobble(black_box(&mut fb), black_box(&config));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_wobble);
criterion_main!(benches);
