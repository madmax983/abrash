use abrash::experimental::tunnel::apply_tunnel;
use abrash::framebuffer::Framebuffer;
use abrash::texture::Texture;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_tunnel(c: &mut Criterion) {
    let mut fb = Framebuffer::new(800, 600).expect("Failed to create FB");
    let tex = Texture::new(256, 256).expect("Failed to create tex");

    let mut group = c.benchmark_group("tunnel");
    group.bench_function("apply_tunnel_800x600", |b| {
        b.iter(|| {
            apply_tunnel(black_box(&mut fb), black_box(1.5), black_box(&tex));
        });
    });
    group.finish();
}

criterion_group!(benches, bench_tunnel);
criterion_main!(benches);
