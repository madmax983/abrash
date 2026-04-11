use abrash::framebuffer::Framebuffer;

#[cfg(feature = "nova")]
use abrash::experimental::pencil::apply_pencil_sketch;

#[cfg(feature = "nova")]
#[test]
fn test_pencil_sketch_basic() {
    let mut fb = Framebuffer::new(3, 3).unwrap();
    for y in 0..3 {
        for x in 0..3 {
            let color = 0xFF000000 | (0x80 << 16) | (0x40 << 8) | 0x20;
            fb.set_pixel(x, y, color);
        }
    }
    apply_pencil_sketch(&mut fb);
    let center_pixel = fb.get_pixel(1, 1).unwrap();
    let r = (center_pixel >> 16) & 0xFF;
    let g = (center_pixel >> 8) & 0xFF;
    let b = center_pixel & 0xFF;
    assert_eq!(r, g, "Red and Green channels should be equal in grayscale output");
    assert_eq!(g, b, "Green and Blue channels should be equal in grayscale output");
}

#[cfg(feature = "nova")]
#[test]
fn test_pencil_sketch_too_small() {
    let mut fb = Framebuffer::new(2, 2).unwrap();
    fb.set_pixel(0, 0, 0xFFFFFFFF);
    apply_pencil_sketch(&mut fb);
    assert_eq!(fb.get_pixel(0, 0).unwrap(), 0xFFFFFFFF, "Should not modify if too small");
}
