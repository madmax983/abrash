use abrash_core::framebuffer::Framebuffer;
use abrash_core::math::{Mat4, Vec3};
use abrash_core::zbuffer::ZBuffer;
use abrash_render::experimental::hex_mosaic::{HexMosaicConfig, apply_hex_mosaic};
use abrash_render::rasterizer::fill_triangle_3d;

fn main() {
    let width = 640;
    let height = 480;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    // Draw some simple geometry
    fb.clear(0xFF_222222);
    zb.clear();

    let view_proj = Mat4::identity();

    // Triangle 1 (Red)
    fill_triangle_3d(
        &mut fb,
        &mut zb,
        view_proj.transform_point(Vec3::new(-0.5, -0.5, 0.5)),
        view_proj.transform_point(Vec3::new(0.5, -0.5, 0.5)),
        view_proj.transform_point(Vec3::new(0.0, 0.5, 0.5)),
        0xFF_FF0000,
    );

    // Apply filter
    let config = HexMosaicConfig {
        radius: 12.0,
        draw_borders: true,
        border_color: 0xFF_000000,
        border_thickness: 1.5,
    };
    apply_hex_mosaic(&mut fb, &config);

    fb.export_ppm("hex_mosaic_demo.ppm").unwrap();
    println!("Exported hex_mosaic_demo.ppm");
}
