//! Pixel buffer management.
//!
//! Framebuffer stores pixels in row-major order as 32-bit RGBA values.
//! Color format: 0xAARRGGBB (little-endian: BB GG RR AA in memory).

pub struct Framebuffer {
    pixels: Vec<u32>,
    width: u32,
    height: u32,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        if width > i32::MAX as u32 || height > i32::MAX as u32 {
            return Err("Buffer dimensions too large (max i32::MAX)");
        }

        let size = (width as u64)
            .checked_mul(height as u64)
            .filter(|&s| s <= u32::MAX as u64)
            .ok_or("Buffer size overflow")? as usize;

        Ok(Self {
            pixels: vec![0xFF00_0000; size], // Black with full alpha
            width,
            height,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn as_slice(&self) -> &[u32] {
        &self.pixels
    }

    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        &mut self.pixels
    }

    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return; // Bounds check - silently ignore out of bounds
        }

        let index = (y as u32 * self.width + x as u32) as usize;
        self.pixels[index] = color;
    }

    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> Option<u32> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }

        let index = (y as u32 * self.width + x as u32) as usize;
        Some(self.pixels[index])
    }

    /// Set pixel color without bounds checking.
    ///
    /// # Safety
    ///
    /// Caller must ensure x and y are within bounds.
    pub unsafe fn set_pixel_unchecked(&mut self, x: usize, y: usize, color: u32) {
        let idx = y * self.width as usize + x;
        // SAFETY: Caller guarantees bounds
        unsafe {
            *self.pixels.get_unchecked_mut(idx) = color;
        }
    }

    /// Clear a rectangular region
    pub fn clear_rect(&mut self, x: i32, y: i32, width: u32, height: u32, color: u32) {
        let x = x.max(0) as u32;
        let y = y.max(0) as u32;
        let x_end = (x + width).min(self.width);
        let y_end = (y + height).min(self.height);

        for row in y..y_end {
            let start = (row * self.width + x) as usize;
            let end = (row * self.width + x_end) as usize;
            self.pixels[start..end].fill(color);
        }
    }
}

/// Blend source color onto destination color using alpha blending.
/// src and dst are 0xAARRGGBB
/// Formula: out = src * alpha + dst * (1 - alpha)
#[inline(always)]
pub fn blend_colors(src: u32, dst: u32) -> u32 {
    let alpha = (src >> 24) & 0xFF;

    if alpha == 255 {
        return src;
    }
    if alpha == 0 {
        return dst;
    }

    // Optimization: SWAR (SIMD Within A Register) for parallel channel blending
    // This assumes alpha is 0..255. We map it to 0..256 for easier division (>> 8)
    // alpha_adj = alpha + 1
    // But standard (src * a + dst * (255-a)) / 255 is slow.
    // Approximate: (src * a + dst * (256-a)) >> 8

    let a = alpha + 1;
    let inv_a = 256 - a;

    let rb_src = src & 0x00FF00FF;
    let g_src = (src >> 8) & 0x00FF00FF;

    let rb_dst = dst & 0x00FF00FF;
    let g_dst = (dst >> 8) & 0x00FF00FF;

    let rb = ((rb_src * a + rb_dst * inv_a) >> 8) & 0x00FF00FF;
    let g = ((g_src * a + g_dst * inv_a) >> 8) & 0x00FF00FF;

    // Result alpha is saturated to 255 (opaque framebuffer assumption) or blended
    // For standard back-to-front compositing on opaque background, we usually set A=255.
    0xFF000000 | rb | (g << 8)
}
