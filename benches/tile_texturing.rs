use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec2, Vec3};
use abrash::mesh::Mesh;
use abrash::rasterizer::{fill_triangle_textured, FilterMode, Texture};
use abrash::tile_renderer::{TexturedClipTriangle, TileRenderer};
use abrash::zbuffer::ZBuffer;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

/// Generate overlapping textured triangles
fn generate_overlapping_textured_triangles(count: usize) -> Vec<TexturedClipTriangle> {
    let mut tris = Vec::with_capacity(count);
    for i in 0..count {
        let t = i as f32 * 0.1;
        let offset_x = (t * 1.7).sin() * 0.15;
        let offset_y = (t * 2.3).cos() * 0.15;
        let depth = 3.0 + (t * 0.5).sin() * 2.0;

        // UVs: Simple mapping (0,0), (0,1), (1,1)
        let v0 = ((Vec3::new(offset_x, 0.5 + offset_y, depth), depth), Vec2::new(0.5, 0.0));
        let v1 = ((Vec3::new(-0.5 + offset_x, -0.5 + offset_y, depth), depth), Vec2::new(0.0, 1.0));
        let v2 = ((Vec3::new(0.5 + offset_x, -0.5 + offset_y, depth), depth), Vec2::new(1.0, 1.0));

        tris.push((v0, v1, v2));
    }
    tris
}

struct Resolution {
    name: &'static str,
    width: u32,
    height: u32,
}

const RESOLUTIONS: &[Resolution] = &[
    Resolution {
        name: "800x600",
        width: 800,
        height: 600,
    },
    Resolution {
        name: "1920x1080",
        width: 1920,
        height: 1080,
    },
    Resolution {
        name: "3840x2160",
        width: 3840,
        height: 2160,
    },
];

const TRIANGLE_COUNTS: &[usize] = &[10, 50, 100];

fn bench_textured_tiled(c: &mut Criterion) {
    let tris_by_count: Vec<(usize, Vec<TexturedClipTriangle>)> = TRIANGLE_COUNTS
        .iter()
        .map(|&n| (n, generate_overlapping_textured_triangles(n)))
        .collect();

    let tex = Texture::checkered(256, 256, 0xFFFF_FFFF, 0xFF00_0000).unwrap();

    for res in RESOLUTIONS {
        let mut group = c.benchmark_group(format!("textured_tiled_{}", res.name));
        let mut fb = Framebuffer::new(res.width, res.height).unwrap();
        let mut zb = ZBuffer::new(res.width, res.height).unwrap();
        let mut tr = TileRenderer::new(res.width, res.height);

        for (count, tris) in &tris_by_count {
            group.bench_with_input(BenchmarkId::from_parameter(count), tris, |b, tris| {
                b.iter(|| {
                    zb.clear();
                    // This method doesn't exist yet - it's the target of our TDD
                    tr.render_batch_textured(&mut fb, &mut zb, black_box(tris), &tex);
                });
            });
        }
        group.finish();
    }
}

fn bench_textured_scanline(c: &mut Criterion) {
    let tris_by_count: Vec<(usize, Vec<TexturedClipTriangle>)> = TRIANGLE_COUNTS
        .iter()
        .map(|&n| (n, generate_overlapping_textured_triangles(n)))
        .collect();

    let tex = Texture::checkered(256, 256, 0xFFFF_FFFF, 0xFF00_0000).unwrap();

    for res in RESOLUTIONS {
        let mut group = c.benchmark_group(format!("textured_scanline_{}", res.name));
        let mut fb = Framebuffer::new(res.width, res.height).unwrap();
        let mut zb = ZBuffer::new(res.width, res.height).unwrap();

        for (count, tris) in &tris_by_count {
            group.bench_with_input(BenchmarkId::from_parameter(count), tris, |b, tris| {
                b.iter(|| {
                    zb.clear();
                    for &(v0, v1, v2) in black_box(tris) {
                        fill_triangle_textured(&mut fb, &mut zb, v0, v1, v2, &tex);
                    }
                });
            });
        }
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_textured_tiled,
    bench_textured_scanline,
);
criterion_main!(benches);
