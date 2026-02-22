use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::rasterizer::fill_triangle_reflection;
use abrash::skybox::Cubemap;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;
use std::time::Instant;

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Create a skybox
    let mut tex_x_pos = Texture::new(256, 256).unwrap();
    tex_x_pos.pixels_mut().fill(0xFF0000FF); // Red
    let mut tex_x_neg = Texture::new(256, 256).unwrap();
    tex_x_neg.pixels_mut().fill(0xFF00FF00); // Green
    let mut tex_y_pos = Texture::new(256, 256).unwrap();
    tex_y_pos.pixels_mut().fill(0xFFFF0000); // Blue
    let mut tex_y_neg = Texture::new(256, 256).unwrap();
    tex_y_neg.pixels_mut().fill(0xFFFFFF00); // Cyan
    let mut tex_z_pos = Texture::new(256, 256).unwrap();
    tex_z_pos.pixels_mut().fill(0xFFFF00FF); // Magenta
    let mut tex_z_neg = Texture::new(256, 256).unwrap();
    tex_z_neg.pixels_mut().fill(0xFF00FFFF); // Yellow

    let cubemap = Cubemap::new([
        tex_x_pos, tex_x_neg, tex_y_pos, tex_y_neg, tex_z_pos, tex_z_neg,
    ]);

    // Define a cube
    let vertices = [
        // Front
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        // Back
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, -1.0),
    ];

    let indices = [
        0, 1, 2, 0, 2, 3, // Front
        5, 4, 7, 5, 7, 6, // Back
        4, 0, 3, 4, 3, 7, // Left
        1, 5, 6, 1, 6, 2, // Right
        3, 2, 6, 3, 6, 7, // Top
        4, 5, 1, 4, 1, 0, // Bottom
    ];

    let camera_pos = Vec3::new(0.0, 0.0, 3.0);
    let target = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);

    let view = Mat4::look_at(camera_pos, target, up);
    let proj = Mat4::perspective(1.0, width as f32 / height as f32, 0.1, 100.0);
    let view_proj = view * proj;

    let start = Instant::now();
    let frames = 100;

    for i in 0..frames {
        let angle = (i as f32) * 0.05;
        let model = Mat4::rotation_y(angle) * Mat4::rotation_x(angle * 0.5);
        let mvp = model * view_proj;

        fb.clear(0xFF333333);
        zb.clear();

        for (i, chunk) in indices.chunks(3).enumerate() {
            let i0 = chunk[0];
            let i1 = chunk[1];
            let i2 = chunk[2];

            let v0_local = vertices[i0];
            let v1_local = vertices[i1];
            let v2_local = vertices[i2];

            // Transform to World
            let (w0, _) = model.transform_point(v0_local);
            let (w1, _) = model.transform_point(v1_local);
            let (w2, _) = model.transform_point(v2_local);

            // Clip Space
            let (c0, cw0) = view_proj.transform_point(w0);
            let (c1, cw1) = view_proj.transform_point(w1);
            let (c2, cw2) = view_proj.transform_point(w2);

            // Normals
            // For a cube, face normals are sufficient if we want flat shading look,
            // but for reflection mapping we want smooth normals usually.
            // Let's use vertex normals (normalized local position for a unit cube centered at origin).
            let n0_local = v0_local.normalize();
            let n1_local = v1_local.normalize();
            let n2_local = v2_local.normalize();

            // Transform normals (Rotation only part of Model Matrix)
            // Mat4 layout:
            // m[0][0] m[0][1] m[0][2] ...
            // upper 3x3 is rotation/scale.
            let rotate_n = |n: Vec3| -> Vec3 {
                let x = n.x * model.m[0][0] + n.y * model.m[1][0] + n.z * model.m[2][0];
                let y = n.x * model.m[0][1] + n.y * model.m[1][1] + n.z * model.m[2][1];
                let z = n.x * model.m[0][2] + n.y * model.m[1][2] + n.z * model.m[2][2];
                Vec3::new(x, y, z).normalize()
            };

            let n0 = rotate_n(n0_local);
            let n1 = rotate_n(n1_local);
            let n2 = rotate_n(n2_local);

            fill_triangle_reflection(
                &mut fb,
                &mut zb,
                ((c0, cw0), n0, w0),
                ((c1, cw1), n1, w1),
                ((c2, cw2), n2, w2),
                camera_pos,
                &cubemap,
            );
        }
    }

    let duration = start.elapsed();
    println!("Rendered {} frames in {:?}", frames, duration);
    println!("FPS: {:.2}", frames as f64 / duration.as_secs_f64());
}
