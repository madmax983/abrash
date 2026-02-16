use abrash::framebuffer::Framebuffer;
use abrash::math::{Vec2, Vec3, Vec4};
use abrash::rasterizer::fill_triangle_normal_mapped;
use abrash::texture::{FilterMode, Texture};
use abrash::zbuffer::ZBuffer;

#[test]
fn test_normal_mapping_simd_correctness() {
    let width = 100;
    let height = 100;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Create a simple texture (white)
    let mut tex = Texture::new(32, 32).unwrap();
    for i in 0..32 * 32 {
        tex.pixels[i] = 0xFFFFFFFF;
    }
    tex.filter_mode = FilterMode::Nearest;

    // Create a normal map (flat normal = 0,0,1 -> 128,128,255)
    let mut nm = Texture::new(32, 32).unwrap();
    for i in 0..32 * 32 {
        nm.pixels[i] = 0xFF8080FF; // roughly (0, 0, 1) in tangent space
    }
    nm.filter_mode = FilterMode::Nearest;

    // Triangle vertices (clip space, w=1)
    let v0 = ((Vec3::new(-0.5, -0.5, 0.5), 1.0), Vec2::new(0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec4::new(1.0, 0.0, 0.0, 1.0));
    let v1 = ((Vec3::new( 0.5, -0.5, 0.5), 1.0), Vec2::new(1.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec4::new(1.0, 0.0, 0.0, 1.0));
    let v2 = ((Vec3::new( 0.0,  0.5, 0.5), 1.0), Vec2::new(0.5, 1.0), Vec3::new(0.0, 0.0, 1.0), Vec4::new(1.0, 0.0, 0.0, 1.0));

    // Light direction (pointing towards -Z, so directly at the surface)
    // Surface normal is +Z (0,0,1). Light comes from +Z to -Z.
    // L vector is vector TO light, so -light_dir.
    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let light_color = Vec3::new(1.0, 1.0, 1.0);
    let ambient = Vec3::new(0.1, 0.1, 0.1);

    // Run the function
    fill_triangle_normal_mapped(
        &mut fb,
        &mut zb,
        v0,
        v1,
        v2,
        &tex,
        &nm,
        light_dir,
        light_color,
        ambient,
    );

    // Check center pixel (50, 50)
    // Expected color: Ambient (0.1) + Diffuse (1.0 * dot(N, L))
    // N = (0,0,1), L = (0,0,1) -> dot = 1.0
    // Total = 1.1 -> clamped to 1.0 -> White (0xFFFFFFFF)
    // But since light_color is multiplied by intensity, max is 1.0+0.1 = 1.1 -> clamped.

    // Wait, shader logic:
    // intensity = dot(N, L) * inv_len ...
    // diffuse_total = pre_diffuse_color * diffuse_sample * intensity
    // final = ambient + diffuse_total

    // If N=(0,0,1), L=(0,0,1), dot=1.0. intensity=1.0.
    // diffuse = 1.0 * 1.0 * 1.0 = 1.0.
    // final = 0.1 + 1.0 = 1.1 -> clamped to 1.0.

    // Let's check a pixel that should be lit.
    let center_pixel = fb.get_pixel(50, 50).unwrap();

    // It should be white.
    assert_eq!(center_pixel, 0xFFFFFFFF, "Center pixel should be white");
}
