use criterion::{black_box, criterion_group, criterion_main, Criterion};
use abrash_core::texture::{laplacian_blend_textures, Texture};

fn bench_laplacian_blend(c: &mut Criterion) {
    let mut tex0 = Texture::new(256, 256).unwrap();
    let mut tex1 = Texture::new(256, 256).unwrap();
    let mut mask = Texture::new(256, 256).unwrap();
    tex0.generate_mipmaps();
    tex1.generate_mipmaps();
    mask.generate_mipmaps();

    c.bench_function("laplacian_blend", |b| {
        b.iter(|| {
            laplacian_blend_textures(
                black_box(&tex0),
                black_box(&tex1),
                black_box(&mask),
                black_box(0.5),
                black_box(0.5),
                black_box(5),
                black_box(0.0),
            )
        })
    });
}

criterion_group!(benches, bench_laplacian_blend);
criterion_main!(benches);
