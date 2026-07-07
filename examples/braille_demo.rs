use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::braille::BrailleConverter;

fn main() {
    let mut fb = Framebuffer::new(80, 40).unwrap();
    fb.clear(0xFF_000000);
    for py in 10..30 {
        for px in 20..60 {
             fb.set_pixel(px, py, 0xFF_FFFFFF);
        }
    }

    let converter = BrailleConverter::new(&fb);
    println!("{}", converter.to_string());
}
