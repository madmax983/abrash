use abrash::texture::Texture;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_get_pixel_texel(c: &mut Criterion) {
    let texture = Texture::new(256, 256).unwrap();
    let width = texture.width as i32;
    let height = texture.height as i32;

    c.bench_function("texture_get_pixel_texel_center", |b| {
        b.iter(|| {
            // Sample center pixels (likely in bounds)
            for y in 100..150 {
                for x in 100..150 {
                    black_box(texture.get_pixel_texel(black_box(x), black_box(y)));
                }
            }
        });
    });

    c.bench_function("texture_get_pixel_texel_edge", |b| {
        b.iter(|| {
            // Sample edge/out of bounds pixels
            for y in -10..40 {
                for x in -10..40 {
                    black_box(texture.get_pixel_texel(black_box(x), black_box(y)));
                }
            }
        });
    });
}

criterion_group!(benches, bench_get_pixel_texel);
criterion_main!(benches);
