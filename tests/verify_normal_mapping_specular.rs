use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3, Vec4};
use abrash::rasterizer::fill_triangle_normal_mapped;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_normal_mapping_specular_compilation() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let diffuse_map = Texture::checkered(2, 2, 0xFFFFFFFF, 0xFFFFFFFF).unwrap();
    let normal_map = Texture::checkered(2, 2, 0xFFFF8080, 0xFFFF8080).unwrap(); // Flat normal

    // View Vector (World Space)
    // Camera at (0, 0, 5), looking at origin.
    // Vertex at (0, 0, 0).
    // View Vector = Camera - Vertex = (0, 0, 5) - (0, 0, 0) = (0, 0, 5).
    let view_dir = Vec3::new(0.0, 0.0, 1.0).normalize();

    // Vertices: ((Pos, W), UV, Normal, Tangent, ViewDir)
    let v0 = (
        (Vec3::new(0.0, 0.5, 0.0), 1.0),
        Vec2::new(0.5, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec4::new(1.0, 0.0, 0.0, 1.0),
        view_dir, // New Argument
    );

    let v1 = (
        (Vec3::new(-0.5, -0.5, 0.0), 1.0),
        Vec2::new(0.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec4::new(1.0, 0.0, 0.0, 1.0),
        view_dir, // New Argument
    );

    let v2 = (
        (Vec3::new(0.5, -0.5, 0.0), 1.0),
        Vec2::new(1.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec4::new(1.0, 0.0, 0.0, 1.0),
        view_dir, // New Argument
    );

    let light_dir = Vec3::new(0.0, 0.0, -1.0).normalize();
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let ambient = Vec3::new(0.1, 0.1, 0.1);

    // New Arguments
    let specular_color = Vec3::new(1.0, 1.0, 1.0);
    let shininess = 32.0;

    fill_triangle_normal_mapped(
        &mut fb,
        &mut zb,
        v0,
        v1,
        v2,
        &diffuse_map,
        &normal_map,
        light_dir,
        light_color,
        ambient,
        specular_color, // New
        shininess,      // New
    );
}
