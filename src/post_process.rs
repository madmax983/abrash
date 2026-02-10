use crate::framebuffer::Framebuffer;

/// Applies a grayscale filter to the framebuffer in-place.
///
/// Uses a fixed-point approximation of the luminance formula:
/// `Y = 0.299*R + 0.587*G + 0.114*B`
///
/// Approximated as: `Y = (77*R + 150*G + 29*B) >> 8`
pub fn apply_grayscale(fb: &mut Framebuffer) {
    let pixels = fb.as_mut_slice();
    for pixel in pixels.iter_mut() {
        // Format: 0xAARRGGBB
        let p = *pixel;
        let r = (p >> 16) & 0xFF;
        let g = (p >> 8) & 0xFF;
        let b = p & 0xFF;

        // Fixed-point luminance calculation
        let luminance = (77 * r + 150 * g + 29 * b) >> 8;

        // Preserve Alpha, set RGB to luminance
        *pixel = (p & 0xFF00_0000) | (luminance << 16) | (luminance << 8) | luminance;
    }
}

/// Simulates CRT scanlines by darkening every odd row.
pub fn apply_scanlines(fb: &mut Framebuffer) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;
    let pixels = fb.as_mut_slice();

    // Iterate over odd rows only
    for y in (1..height).step_by(2) {
        let start = y * width;
        let end = start + width;
        let row = &mut pixels[start..end];
        for pixel in row.iter_mut() {
            let p = *pixel;
            // Halve RGB components: (color >> 1) & mask
            // Preserve Alpha: (p & 0xFF00_0000)
            *pixel = ((p >> 1) & 0x7F7F_7F7F) | (p & 0xFF00_0000);
        }
    }
}
