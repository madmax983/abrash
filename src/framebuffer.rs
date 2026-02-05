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
        let dst = self.pixels[index];
        self.pixels[index] = blend_colors(color, dst);
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
            let dst = *self.pixels.get_unchecked(idx);
            *self.pixels.get_unchecked_mut(idx) = blend_colors(color, dst);
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

/// Helper function to blend two colors (src over dst)
/// Optimized using SWAR (SIMD Within A Register) to blend RB and GA channels in parallel.
#[inline(always)]
pub fn blend_colors(src: u32, dst: u32) -> u32 {
    let alpha = (src >> 24) & 0xFF;
    if alpha == 255 {
        return src;
    }
    if alpha == 0 {
        return dst;
    }

    // Scale alpha to 0..256 range for >> 8 optimization
    // 255 -> 256, 128 -> 129, 0 -> 0
    let a = alpha + (alpha >> 7);
    let inv_a = 256 - a;

    let rb_s = src & 0x00FF00FF;
    let g_s = (src >> 8) & 0x00FF00FF;

    let rb_d = dst & 0x00FF00FF;
    let g_d = (dst >> 8) & 0x00FF00FF;

    let rb = ((rb_s * a + rb_d * inv_a) >> 8) & 0x00FF00FF;
    let g = ((g_s * a + g_d * inv_a) >> 8) & 0x00FF00FF;

    (rb | (g << 8)) | 0xFF000000
}
