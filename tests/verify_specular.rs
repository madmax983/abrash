use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3, Vec4};
use abrash::rasterizer::fill_triangle_normal_mapped;
use abrash::texture::Texture;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_specular_signature() {
    let mut fb = Framebuffer::new(100, 100).unwrap();
    let mut zb = ZBuffer::new(100, 100).unwrap();
    let texture = Texture::checkered(32, 32, 0xFFFFFFFF, 0xFF000000).unwrap();
    let normal_map = Texture::checkered(32, 32, 0xFF8080FF, 0xFF8080FF).unwrap();

    let v0 = ((Vec3::new(0.0, 0.0, 0.0), 1.0), Vec2::new(0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec4::new(1.0, 0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 0.0));
    let v1 = ((Vec3::new(10.0, 0.0, 0.0), 1.0), Vec2::new(1.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec4::new(1.0, 0.0, 0.0, 1.0), Vec3::new(10.0, 0.0, 0.0));
    let v2 = ((Vec3::new(0.0, 10.0, 0.0), 1.0), Vec2::new(0.0, 1.0), Vec3::new(0.0, 0.0, 1.0), Vec4::new(1.0, 0.0, 0.0, 1.0), Vec3::new(0.0, 10.0, 0.0));

    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let ambient = Vec3::new(0.1, 0.1, 0.1);

    // New parameters
    let view_pos = Vec3::new(0.0, 0.0, 5.0);
    let shininess = 32.0;
    let specular_strength = Vec3::new(1.0, 1.0, 1.0);

    fill_triangle_normal_mapped(
        &mut fb,
        &mut zb,
        v0,
        v1,
        v2,
        &texture,
        &normal_map,
        light_dir,
        light_color,
        ambient,
        view_pos,
        shininess,
        specular_strength,
    );
}
