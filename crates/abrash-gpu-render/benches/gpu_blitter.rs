use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

use abrash_core::blitter::SrcRect;
use abrash_core::framebuffer::Framebuffer;
use abrash_core::texture::Texture;
use abrash_gpu_render::blitter::{BlitMode, GpuBlitter};
use abrash_gpu_render::device::{GpuDevice, GpuDeviceConfig};

fn headless_device() -> GpuDevice {
    GpuDevice::new_headless(&GpuDeviceConfig::headless()).expect("GPU required for benchmarks")
}

fn bench_gpu_blit_batch_scaling(c: &mut Criterion) {
    let gpu = headless_device();
    let sprite_counts: &[u32] = &[10, 50, 100, 500, 1000];

    let mut group = c.benchmark_group("gpu_blit_batch_scaling");

    for &count in sprite_counts {
        let mut blitter = GpuBlitter::new(&gpu, 1920, 1080);

        let mut tex = Texture::new(64, 64).unwrap();
        for y in 0..64u32 {
            for x in 0..64u32 {
                tex.set_pixel(x, y, 0xFFFF_0000);
            }
        }
        let atlas = blitter.upload_atlas(&tex);
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 64,
            h: 64,
        };

        group.throughput(Throughput::Elements(u64::from(count)));
        group.bench_with_input(
            BenchmarkId::new("opaque_flush_to_fb", count),
            &count,
            |b, &count| {
                let mut fb = Framebuffer::new(1920, 1080).unwrap();
                b.iter(|| {
                    for i in 0..count {
                        let x = (i % 30) as i32 * 64;
                        let y = (i / 30) as i32 * 64;
                        blitter.queue(atlas, src, x, y, BlitMode::Opaque);
                    }
                    blitter.flush_to_framebuffer(&mut fb);
                });
            },
        );
    }
    group.finish();
}

fn bench_gpu_vs_cpu_alpha(c: &mut Criterion) {
    let gpu = headless_device();
    let mut group = c.benchmark_group("gpu_vs_cpu_500_alpha");

    let mut tex = Texture::new(64, 64).unwrap();
    for y in 0..64u32 {
        for x in 0..64u32 {
            tex.set_pixel(x, y, 0x80FF_0000);
        }
    }
    let src = SrcRect {
        x: 0,
        y: 0,
        w: 64,
        h: 64,
    };

    // GPU path
    {
        let mut blitter = GpuBlitter::new(&gpu, 1920, 1080);
        let atlas = blitter.upload_atlas(&tex);
        let mut fb = Framebuffer::new(1920, 1080).unwrap();
        group.bench_function("gpu_alpha_500", |b| {
            b.iter(|| {
                for i in 0..500u32 {
                    let x = (i % 30) as i32 * 64;
                    let y = (i / 30) as i32 * 64;
                    blitter.queue(atlas, src, x, y, BlitMode::Alpha);
                }
                blitter.flush_to_framebuffer(&mut fb);
            });
        });
    }

    // CPU path
    {
        let mut fb = Framebuffer::new(1920, 1080).unwrap();
        group.bench_function("cpu_alpha_500", |b| {
            b.iter(|| {
                for i in 0..500u32 {
                    let x = (i % 30) as i32 * 64;
                    let y = (i / 30) as i32 * 64;
                    abrash_core::blitter::blit_alpha(&mut fb, &tex, src, x, y);
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_gpu_blit_batch_scaling,
    bench_gpu_vs_cpu_alpha
);
criterion_main!(benches);
