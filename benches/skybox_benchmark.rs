use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::skybox::{Cubemap, draw_skybox};
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_skybox_render(c: &mut Criterion) {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Create 6 faces (checkerboard)
    // 512x512 textures to simulate reasonable load
    let tex_size = 512;
    let faces = [
        Texture::checkered(tex_size, tex_size, 0xFFFF0000, 0xFFFFFFFF).unwrap(),
        Texture::checkered(tex_size, tex_size, 0xFF00FFFF, 0xFFFFFFFF).unwrap(),
        Texture::checkered(tex_size, tex_size, 0xFF0000FF, 0xFFFFFFFF).unwrap(),
        Texture::checkered(tex_size, tex_size, 0xFFFFFF00, 0xFFFFFFFF).unwrap(),
        Texture::checkered(tex_size, tex_size, 0xFF00FF00, 0xFFFFFFFF).unwrap(),
        Texture::checkered(tex_size, tex_size, 0xFFFF00FF, 0xFFFFFFFF).unwrap(),
    ];
    let cubemap = Cubemap::new(faces);

    // Setup camera
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 0.0), // Camera at center
        Vec3::new(1.0, 0.0, 0.0),  // Look at +X
        Vec3::new(0.0, 1.0, 0.0),  // Up
    );
    let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 100.0);

    c.bench_function("skybox_render_full_hd", |b| {
        b.iter(|| {
            fb.clear(0xFF000000);
            zb.clear();
            draw_skybox(
                black_box(&mut fb),
                black_box(&mut zb),
                black_box(view),
                black_box(proj),
                black_box(&cubemap),
            );
        });
    });
}

criterion_group!(benches, bench_skybox_render);
criterion_main!(benches);
