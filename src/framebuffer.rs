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

/// Blends two colors using source alpha.
///
/// Uses SWAR (SIMD Within A Register) optimization to blend in parallel.
/// Assumes alpha is in the range [0, 255].
///
/// * `src` - Source color (foreground)
/// * `dst` - Destination color (background)
/// * `alpha` - Source alpha (0-255)
#[inline(always)]
pub fn blend_colors(src: u32, dst: u32, alpha: u32) -> u32 {
    let inv_alpha = 256 - alpha;

    // Mask out Red/Blue and Alpha/Green channels
    // 0xFF00FF - keeps R and B
    // 0xFF00FF00 - keeps A and G (shifted)
    let rb_src = src & 0x00FF00FF;
    let ag_src = (src >> 8) & 0x00FF00FF;

    let rb_dst = dst & 0x00FF00FF;
    let ag_dst = (dst >> 8) & 0x00FF00FF;

    // (src * alpha + dst * (256 - alpha)) >> 8
    // We use 256 for inverse to allow bit shift instead of division by 255.
    // This slightly compresses the range but is standard fast approximation.
    // Note: alpha range is 0..256 ideally for this math, but input is 0..255.
    // Mapping 0..255 to 0..256: alpha + (alpha >> 7) is a common approximation.
    // But here we'll just use alpha directly and inv_alpha = 256 - alpha.
    // If alpha=255, inv=1. If alpha=0, inv=256.
    // This implies full transparency (alpha=0) keeps destination fully (dst * 256 >> 8 = dst).
    // Full opacity (alpha=255) gives (src * 255 + dst * 1) >> 8 ~ src.

    let rb = ((rb_src * alpha + rb_dst * inv_alpha) >> 8) & 0x00FF00FF;
    let ag = ((ag_src * alpha + ag_dst * inv_alpha) >> 8) & 0x00FF00FF;

    rb | (ag << 8) | 0xFF000000 // Force alpha to 255 for result
}
