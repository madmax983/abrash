#![cfg(feature = "nova")]

use abrash::experimental::tunnel::apply_tunnel;
use abrash::framebuffer::Framebuffer;
use abrash::texture::Texture;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_tunnel(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).unwrap();
    let mut texture = Texture::new(256, 256).unwrap();
    for y in 0..256 {
        for x in 0..256 {
            texture.set_pixel(x, y, 0xFF_AA_BB_CC);
        }
    }

    c.bench_function("tunnel_800x600", |b| {
        b.iter(|| {
            apply_tunnel(black_box(&mut fb), black_box(1.5), black_box(&texture));
        });
    });
}

criterion_group!(benches, bench_tunnel);
criterion_main!(benches);
