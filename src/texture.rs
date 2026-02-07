//! 2D Texture and sampling primitives.

/// Filter mode for texture sampling.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterMode {
    /// Nearest neighbor interpolation. Fast, but pixelated.
    Nearest,
    /// Bilinear interpolation. Smoother, but slower.
    Bilinear,
}

/// Helper for bilinear interpolation blending using SWAR (SIMD Within A Register)
#[inline(always)]
const fn blend_swar(c0: u32, c1: u32, w: u32, inv_w: u32) -> u32 {
    let rb0 = c0 & 0x00FF_00FF;
    let ag0 = (c0 >> 8) & 0x00FF_00FF;
    let rb1 = c1 & 0x00FF_00FF;
    let ag1 = (c1 >> 8) & 0x00FF_00FF;

    let rb = ((rb0 * inv_w + rb1 * w) >> 8) & 0x00FF_00FF;
    let ag = ((ag0 * inv_w + ag1 * w) >> 8) & 0x00FF_00FF;

    rb | (ag << 8)
}

/// A simple 2D texture.
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
    pub filter_mode: FilterMode,
}

impl Texture {
    /// Creates a new texture with the given dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error if dimensions are zero or the total pixel count overflows `u32`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::texture::Texture;
    ///
    /// let texture = Texture::new(256, 256).unwrap();
    /// assert_eq!(texture.width, 256);
    /// ```
    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        if width == 0 || height == 0 {
            return Err("Texture dimensions must be positive");
        }
        if width > i32::MAX as u32 || height > i32::MAX as u32 {
            return Err("Texture dimensions too large (max i32::MAX)");
        }

        let size = u64::from(width)
            .checked_mul(u64::from(height))
            .filter(|&s| u32::try_from(s).is_ok())
            .ok_or("Texture size overflow")? as usize;

        Ok(Self {
            width,
            height,
            pixels: vec![0xFF00_0000; size],
            filter_mode: FilterMode::Nearest,
        })
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = color;
        }
    }

    /// Sample texture using interpolation mode
    /// u, v are in range [0.0, 1.0]
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash::texture::Texture;
    ///
    /// let mut tex = Texture::new(2, 2).unwrap();
    /// tex.set_pixel(0, 0, 0xFFFFFFFF);
    ///
    /// // Sample center of top-left pixel
    /// let color = tex.get_pixel(0.25, 0.25);
    /// assert_eq!(color, 0xFFFFFFFF);
    /// ```
    #[inline]
    #[must_use]
    pub fn get_pixel(&self, u: f32, v: f32) -> u32 {
        match self.filter_mode {
            FilterMode::Nearest => {
                let x = (u * self.width as f32) as i32;
                let y = (v * self.height as f32) as i32;
                self.get_pixel_texel(x, y)
            }
            FilterMode::Bilinear => self.get_pixel_bilinear(u, v),
        }
    }

    /// Sample texture using bilinear interpolation
    #[must_use]
    pub fn get_pixel_bilinear(&self, u: f32, v: f32) -> u32 {
        let w = self.width as f32;
        let h = self.height as f32;
        self.get_pixel_bilinear_texel(u * w, v * h)
    }

    /// Sample texture using bilinear interpolation with texel coordinates
    #[inline]
    #[must_use]
    pub fn get_pixel_bilinear_texel(&self, u_tex: f32, v_tex: f32) -> u32 {
        // Convert to 24.8 fixed point
        // 0.5 in 24.8 is 128
        let u_fixed = (u_tex * 256.0) as i32;
        let v_fixed = (v_tex * 256.0) as i32;
        self.get_pixel_bilinear_fixed(u_fixed, v_fixed)
    }

    /// Sample texture using bilinear interpolation with 24.8 fixed point texel coordinates
    #[inline]
    #[must_use]
    pub fn get_pixel_bilinear_fixed(&self, u_fixed: i32, v_fixed: i32) -> u32 {
        let u_img_fixed = u_fixed - 128;
        let v_img_fixed = v_fixed - 128;

        // Weights (0..256)
        let wx = (u_img_fixed & 0xFF) as u32;
        let wy = (v_img_fixed & 0xFF) as u32;
        let inv_wx = 256 - wx;
        let inv_wy = 256 - wy;

        // Arithmetic shift preserves sign (floor behavior for negative numbers)
        let x0_raw = u_img_fixed >> 8;
        let y0_raw = v_img_fixed >> 8;

        let (c00, c10, c01, c11) = self.fetch_bilinear_neighbors(x0_raw, y0_raw);

        let top = blend_swar(c00, c10, wx, inv_wx);
        let bottom = blend_swar(c01, c11, wx, inv_wx);
        let final_color = blend_swar(top, bottom, wy, inv_wy);

        // Ensure alpha is 0xFF
        final_color | 0xFF00_0000
    }

    #[inline(always)]
    fn fetch_bilinear_neighbors(&self, x0_raw: i32, y0_raw: i32) -> (u32, u32, u32, u32) {
        let w_i32 = self.width as i32 - 1;
        let h_i32 = self.height as i32 - 1;

        // Optimization: Fast path for interior pixels to avoid 4 clamps
        // w_i32 is width - 1. If x0_raw < w_i32, then x0_raw <= width - 2, so x0_raw + 1 <= width - 1.
        if x0_raw >= 0 && x0_raw < w_i32 && y0_raw >= 0 && y0_raw < h_i32 {
            let x0 = x0_raw as usize;
            let y0 = y0_raw as usize;
            let width_usize = self.width as usize;
            let row0 = y0 * width_usize;
            let row1 = row0 + width_usize; // y0 + 1 is valid

            unsafe {
                (
                    *self.pixels.get_unchecked(row0 + x0),
                    *self.pixels.get_unchecked(row0 + x0 + 1),
                    *self.pixels.get_unchecked(row1 + x0),
                    *self.pixels.get_unchecked(row1 + x0 + 1),
                )
            }
        } else {
            let x0 = x0_raw.clamp(0, w_i32) as usize;
            let y0 = y0_raw.clamp(0, h_i32) as usize;
            let x1 = (x0_raw + 1).clamp(0, w_i32) as usize;
            let y1 = (y0_raw + 1).clamp(0, h_i32) as usize;

            let width_usize = self.width as usize;
            let row0 = y0 * width_usize;
            let row1 = y1 * width_usize;

            // SAFETY: We clamped coordinates to valid ranges [0, width-1] / [0, height-1]
            unsafe {
                (
                    *self.pixels.get_unchecked(row0 + x0),
                    *self.pixels.get_unchecked(row0 + x1),
                    *self.pixels.get_unchecked(row1 + x0),
                    *self.pixels.get_unchecked(row1 + x1),
                )
            }
        }
    }

    #[must_use]
    pub fn get_pixel_texel(&self, x: i32, y: i32) -> u32 {
        let x = x.clamp(0, self.width as i32 - 1) as usize;
        let y = y.clamp(0, self.height as i32 - 1) as usize;
        unsafe { *self.pixels.get_unchecked(y * self.width as usize + x) }
    }

    /// Create a checkerboard texture.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `Texture::new` call fails.
    pub fn checkered(width: u32, height: u32, c1: u32, c2: u32) -> Result<Self, &'static str> {
        let mut tex = Self::new(width, height)?;
        // Scale checks based on size, defaulting to 8x8 blocks
        let block_w = (width / 8).max(1);
        let block_h = (height / 8).max(1);

        for y in 0..height {
            for x in 0..width {
                let check = ((x / block_w) + (y / block_h)) & 1 == 0;
                tex.set_pixel(x, y, if check { c1 } else { c2 });
            }
        }
        Ok(tex)
    }
}
