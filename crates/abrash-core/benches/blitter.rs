use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

use abrash_core::blitter::{SrcRect, blit_alpha, blit_colorkey, blit_opaque};
use abrash_core::framebuffer::Framebuffer;
use abrash_core::texture::Texture;

fn make_opaque_texture(w: u32, h: u32) -> Texture {
    let mut tex = Texture::new(w, h).unwrap();
    for y in 0..h {
        for x in 0..w {
            tex.set_pixel(x, y, 0xFFFF_0000);
        }
    }
    tex
}

fn make_alpha_texture(w: u32, h: u32, alpha: u8) -> Texture {
    let mut tex = Texture::new(w, h).unwrap();
    let color = (alpha as u32) << 24 | 0x00FF_0000;
    for y in 0..h {
        for x in 0..w {
            tex.set_pixel(x, y, color);
        }
    }
    tex
}

fn bench_blit_opaque(c: &mut Criterion) {
    let mut group = c.benchmark_group("blit_opaque");
    for size in [16u32, 32, 64, 128] {
        let pixels = u64::from(size) * u64::from(size);
        group.throughput(Throughput::Elements(pixels));
        let tex = make_opaque_texture(size, size);
        let mut fb = Framebuffer::new(1024, 768).unwrap();
        let src = SrcRect {
            x: 0,
            y: 0,
            w: size,
            h: size,
        };

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                blit_opaque(black_box(&mut fb), black_box(&tex), src, 100, 100);
            });
        });
    }
    group.finish();
}

fn bench_blit_colorkey(c: &mut Criterion) {
    let mut group = c.benchmark_group("blit_colorkey");
    for size in [16u32, 32, 64, 128] {
        let pixels = u64::from(size) * u64::from(size);
        group.throughput(Throughput::Elements(pixels));
        let tex = make_opaque_texture(size, size);
        let mut fb = Framebuffer::new(1024, 768).unwrap();
        let src = SrcRect {
            x: 0,
            y: 0,
            w: size,
            h: size,
        };

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                blit_colorkey(
                    black_box(&mut fb),
                    black_box(&tex),
                    src,
                    100,
                    100,
                    0xFFFF_00FF,
                );
            });
        });
    }
    group.finish();
}

fn bench_blit_alpha(c: &mut Criterion) {
    let mut group = c.benchmark_group("blit_alpha");
    for size in [16u32, 32, 64, 128] {
        let pixels = u64::from(size) * u64::from(size);
        group.throughput(Throughput::Elements(pixels));
        let tex = make_alpha_texture(size, size, 0x80);
        let mut fb = Framebuffer::new(1024, 768).unwrap();
        let src = SrcRect {
            x: 0,
            y: 0,
            w: size,
            h: size,
        };

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                blit_alpha(black_box(&mut fb), black_box(&tex), src, 100, 100);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_blit_opaque,
    bench_blit_colorkey,
    bench_blit_alpha
);
criterion_main!(benches);
