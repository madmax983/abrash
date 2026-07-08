use abrash::experimental::braille::BrailleConverter;
use abrash::framebuffer::Framebuffer;
use abrash::math::{Mat4, Vec3};
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;

fn main() {
    // 80x24 terminal chars (braille is 2x4 pixels per char) -> 160x96 pixels
    let width = 160;
    let height = 96;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 100.0);

    // Animate a simple spinning triangle
    for i in 0..10 {
        let angle = i as f32 * 0.2;
        let model = Mat4::rotation_y(angle) * Mat4::rotation_x(angle * 0.5);
        let mvp = model * view * proj;

        fb.clear(0xFF_000000);
        zb.clear();

        let v0 = mvp.transform_point(Vec3::new(0.0, 1.0, 0.0));
        let v1 = mvp.transform_point(Vec3::new(-1.0, -1.0, 0.0));
        let v2 = mvp.transform_point(Vec3::new(1.0, -1.0, 0.0));

        fill_triangle_3d(&mut fb, &mut zb, v0, v1, v2, 0xFF_FFFFFF);

        let converter = BrailleConverter::new(&fb);
        let art = converter.to_string();

        // Print with clear screen escape code
        print!("\x1B[2J\x1B[1;1H");
        println!("{art}");

        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
