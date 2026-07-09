use abrash::framebuffer::Framebuffer;
use abrash::math::Mat4;
use abrash::post_process;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::f32::consts::PI;

fn benchmark_grayscale(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    // Fill with a pattern
    fb.clear(0xFFFF_0000);

    c.bench_function("apply_grayscale 1080p", |b| {
        b.iter(|| {
            post_process::apply_grayscale(black_box(&mut fb));
        });
    });
}

fn benchmark_scanlines(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFFFF_FFFF);

    c.bench_function("apply_scanlines 1080p", |b| {
        b.iter(|| {
            post_process::apply_scanlines(black_box(&mut fb));
        });
    });
}

fn benchmark_invert(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFF00_0000);

    c.bench_function("apply_invert 1080p", |b| {
        b.iter(|| {
            post_process::apply_invert(black_box(&mut fb));
        });
    });
}

fn benchmark_sepia(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFFFF_FFFF); // White

    c.bench_function("apply_sepia 1080p", |b| {
        b.iter(|| {
            post_process::apply_sepia(black_box(&mut fb));
        });
    });
}

fn benchmark_chromatic_aberration(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFFFF_FFFF); // White

    c.bench_function("apply_chromatic_aberration 1080p", |b| {
        b.iter(|| {
            post_process::apply_chromatic_aberration(
                black_box(&mut fb),
                black_box(&abrash::post_process::ChromaticAberrationConfig { offset: 5 }),
            );
        });
    });
}

fn benchmark_bloom(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFFFF_FFFF); // White

    let config = post_process::BloomConfig {
        threshold: 200,
        blur_radius: 10,
        intensity: 0.8,
    };
    c.bench_function("apply_bloom 1080p (r=10)", |b| {
        b.iter(|| {
            post_process::apply_bloom(black_box(&mut fb), black_box(&config));
        });
    });
}

fn benchmark_ssao(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();
    let proj = Mat4::perspective(PI / 2.0, 1.0, 0.1, 100.0);

    // Populate buffers
    fb.clear(0xFFFF_FFFF);
    // Fill Z buffer with some data (gradient)
    for y in 0..height {
        for x in 0..width {
            let depth = 0.5 + (x as f32 / width as f32) * 0.4;
            zb.test_and_set(x as i32, y as i32, depth);
        }
    }

    let config = post_process::SsaoConfig {
        radius: 1.0,
        bias: 0.001,
        intensity: 2.0,
    };
    c.bench_function("apply_ssao 1080p", |b| {
        b.iter(|| {
            post_process::apply_ssao(
                black_box(&mut fb),
                black_box(&zb),
                black_box(&proj),
                black_box(&config),
            );
        });
    });
}

fn benchmark_box_blur_f32(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let size = width * height;
    let mut src = vec![0.5; size];
    let mut dest = vec![0.0; size];
    let mut acc = vec![0.0; width];

    c.bench_function("box_blur_f32 1080p", |b| {
        b.iter(|| {
            post_process::box_blur_f32(
                black_box(&mut src),
                black_box(&mut dest),
                black_box(&mut acc),
                black_box(width),
                black_box(height),
            );
        });
    });
}

fn benchmark_box_blur_horizontal(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let size = width * height;
    let src = vec![0u32; size];
    let mut dest = vec![0u32; size];

    c.bench_function("box_blur_horizontal 1080p", |b| {
        b.iter(|| {
            post_process::box_blur_horizontal(
                black_box(&src),
                black_box(&mut dest),
                black_box(width),
                black_box(height),
                black_box(10), // radius
            );
        });
    });
}

fn benchmark_sobel(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    // Fill with a checkerboard pattern to ensure gradients
    for y in 0..height {
        for x in 0..width {
            let color = if (x / 50 + y / 50) % 2 == 0 {
                0xFFFF_FFFF
            } else {
                0xFF00_0000
            };
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_sobel 1080p", |b| {
        b.iter(|| {
            post_process::apply_sobel(black_box(&mut fb));
        });
    });
}

fn benchmark_dof(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Populate buffers
    fb.clear(0xFFFF_FFFF);
    // Fill Z buffer with some data (gradient)
    for y in 0..height {
        for x in 0..width {
            let depth = 0.5 + (x as f32 / width as f32) * 0.4;
            zb.test_and_set(x as i32, y as i32, depth);
        }
    }

    let config = post_process::DepthOfFieldConfig {
        focus_dist: 0.7,
        focus_range: 0.1,
        blur_radius: 5,
    };
    c.bench_function("apply_depth_of_field 1080p", |b| {
        b.iter(|| {
            post_process::apply_depth_of_field(
                black_box(&mut fb),
                black_box(&zb),
                black_box(&config),
            );
        });
    });
}

fn benchmark_vignette(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    fb.clear(0xFFFF_FFFF);

    c.bench_function("apply_vignette 1080p", |b| {
        b.iter(|| {
            post_process::apply_vignette(
                black_box(&mut fb),
                black_box(&post_process::filters::VignetteConfig {
                    intensity: 0.5,
                    roundness: 0.5,
                }),
            );
        });
    });
}

fn benchmark_exposure(c: &mut Criterion) {
    let mut fb = Framebuffer::new(1920, 1080).unwrap();
    c.bench_function("apply_exposure 1080p", |b| {
        b.iter(|| {
            post_process::filters::apply_exposure(black_box(&mut fb), black_box(2.0));
        });
    });
}

fn benchmark_color_adjust(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    for y in 0..height {
        for x in 0..width {
            let r = (x * 4) % 256;
            let g = (y * 4) % 256;
            let b = ((x + y) * 2) % 256;
            let color = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_color_adjust 1080p", |b| {
        b.iter(|| {
            post_process::apply_color_adjust(
                black_box(&mut fb),
                black_box(&post_process::filters::ColorAdjustConfig {
                    brightness: 10,
                    contrast: 1.2,
                }),
            );
        });
    });
}

#[cfg(feature = "nova")]
fn benchmark_pixel_sort(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    // Fill with a pattern
    for y in 0..height {
        for x in 0..width {
            let color = if (x / 50 + y / 50) % 2 == 0 {
                0xFFFF_FFFF
            } else {
                0xFF00_0000
            };
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    let config_horizontal = abrash::experimental::pixel_sort::PixelSortConfig {
        threshold: 0.5,
        vertical: false,
        reverse: false,
    };
    c.bench_function("apply_pixel_sort 1080p horizontal", |b| {
        b.iter(|| {
            abrash::experimental::pixel_sort::apply_pixel_sort(
                black_box(&mut fb),
                black_box(&config_horizontal),
            );
        });
    });

    let config_vertical = abrash::experimental::pixel_sort::PixelSortConfig {
        threshold: 0.5,
        vertical: true,
        reverse: false,
    };
    c.bench_function("apply_pixel_sort 1080p vertical", |b| {
        b.iter(|| {
            abrash::experimental::pixel_sort::apply_pixel_sort(
                black_box(&mut fb),
                black_box(&config_vertical),
            );
        });
    });
}

#[cfg(feature = "nova")]
fn benchmark_halftone(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let mut fb = Framebuffer::new(width, height).unwrap();
    // Fill with a pattern
    for y in 0..height {
        for x in 0..width {
            let color = if (x / 50 + y / 50) % 2 == 0 {
                0xFFFF_FFFF
            } else {
                0xFF00_0000
            };
            fb.set_pixel(x as i32, y as i32, color);
        }
    }

    c.bench_function("apply_halftone 1080p", |b| {
        b.iter(|| {
            abrash::experimental::halftone::apply_halftone(
                black_box(&mut fb),
                black_box(5.0),
                black_box(std::f32::consts::FRAC_PI_4),
            );
        });
    });
}

#[cfg(feature = "nova")]
criterion_group!(
    benches,
    benchmark_grayscale,
    benchmark_scanlines,
    benchmark_invert,
    benchmark_sepia,
    benchmark_chromatic_aberration,
    benchmark_bloom,
    benchmark_ssao,
    benchmark_box_blur_f32,
    benchmark_box_blur_horizontal,
    benchmark_sobel,
    benchmark_dof,
    benchmark_vignette,
    benchmark_color_adjust,
    benchmark_pixel_sort,
    benchmark_halftone,
    benchmark_exposure,
);

#[cfg(not(feature = "nova"))]
criterion_group!(
    benches,
    benchmark_grayscale,
    benchmark_scanlines,
    benchmark_invert,
    benchmark_sepia,
    benchmark_chromatic_aberration,
    benchmark_bloom,
    benchmark_ssao,
    benchmark_box_blur_f32,
    benchmark_box_blur_horizontal,
    benchmark_sobel,
    benchmark_dof,
    benchmark_vignette,
    benchmark_color_adjust,
    benchmark_exposure,
);
criterion_main!(benches);
