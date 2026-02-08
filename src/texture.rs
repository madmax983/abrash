//! 2D Texture representation and sampling.
//!
//! Moving texture logic here allows multiple rasterizers to share it without depending on each other.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// Filter mode for texture sampling.
pub enum FilterMode {
    /// Nearest neighbor interpolation. Fast, but pixelated.
    Nearest,
    /// Bilinear interpolation. Smoother, but slower.
    Bilinear,
    /// Trilinear interpolation (Bilinear between mipmap levels). Smoother at distance.
    Trilinear,
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
    /// Mipmap levels. mips[0] is level 1, mips[1] is level 2, etc.
    /// Level 0 is stored in `pixels`.
    pub mips: Vec<Vec<u32>>,
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
            mips: Vec::new(),
            filter_mode: FilterMode::Nearest,
        })
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = color;
        }
    }

    /// Generate mipmaps for the texture using box filtering.
    pub fn generate_mipmaps(&mut self) {
        self.mips.clear();

        let mut w = self.width;
        let mut h = self.height;

        while w > 1 || h > 1 {
             let src_w = w;
             let src_h = h;
             let dst_w = (w >> 1).max(1);
             let dst_h = (h >> 1).max(1);

             // We can't borrow self.mips inside the loop if we push to it.
             // But we only need to read the previous level.
             // We can use indexing.

             let size = (dst_w * dst_h) as usize;
             let mut dst_pixels = Vec::with_capacity(size);

             // To avoid multiple borrows of self, we'll access pixels via a temporary slice
             // but that's hard because `mips` is inside `self`.
             // Instead, let's just use raw pointers or unsafe for the read, or
             // more safely: swap the source vector out, read it, then put it back? No.

             // Standard Rust workaround: Use indices and `get`
             // or extract the logic to a pure function.

             // Let's use a pure function helper `downsample`.
             let src_pixels = if self.mips.is_empty() {
                 &self.pixels
             } else {
                 &self.mips[self.mips.len() - 1]
             };

             Texture::downsample(src_pixels, src_w, src_h, dst_w, dst_h, &mut dst_pixels);

             self.mips.push(dst_pixels);
             w = dst_w;
             h = dst_h;
        }
    }

    fn downsample(src: &[u32], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32, dst: &mut Vec<u32>) {
        for y in 0..dst_h {
            for x in 0..dst_w {
                let px0 = if src_w > 1 { x * 2 } else { x };
                let py0 = if src_h > 1 { y * 2 } else { y };
                let px1 = if src_w > 1 { (x * 2 + 1).min(src_w - 1) } else { x };
                let py1 = if src_h > 1 { (y * 2 + 1).min(src_h - 1) } else { y };

                let i00 = (py0 * src_w + px0) as usize;
                let i10 = (py0 * src_w + px1) as usize;
                let i01 = (py1 * src_w + px0) as usize;
                let i11 = (py1 * src_w + px1) as usize;

                let c00 = src[i00];
                let c10 = src[i10];
                let c01 = src[i01];
                let c11 = src[i11];

                let a = ((c00 >> 24) + (c10 >> 24) + (c01 >> 24) + (c11 >> 24)) >> 2;
                let r = (((c00 >> 16) & 0xFF) + ((c10 >> 16) & 0xFF) + ((c01 >> 16) & 0xFF) + ((c11 >> 16) & 0xFF)) >> 2;
                let g = (((c00 >> 8) & 0xFF) + ((c10 >> 8) & 0xFF) + ((c01 >> 8) & 0xFF) + ((c11 >> 8) & 0xFF)) >> 2;
                let b = ((c00 & 0xFF) + (c10 & 0xFF) + (c01 & 0xFF) + (c11 & 0xFF)) >> 2;

                dst.push((a << 24) | (r << 16) | (g << 8) | b);
            }
        }
    }

    /// Sample texture using interpolation mode
    /// u, v are in range [0.0, 1.0]
    #[inline]
    #[must_use]
    pub fn get_pixel(&self, u: f32, v: f32) -> u32 {
        self.get_pixel_lod(u, v, 0.0)
    }

    /// Sample texture with Level of Detail (LOD)
    #[inline]
    #[must_use]
    pub fn get_pixel_lod(&self, u: f32, v: f32, lod: f32) -> u32 {
        match self.filter_mode {
            FilterMode::Nearest => {
                let x = (u * self.width as f32) as i32;
                let y = (v * self.height as f32) as i32;
                self.get_pixel_texel(x, y)
            }
            FilterMode::Bilinear => {
                self.get_pixel_bilinear(u, v)
            }
            FilterMode::Trilinear => {
                if self.mips.is_empty() {
                    return self.get_pixel_bilinear(u, v);
                }

                let lod = lod.max(0.0);
                let level = lod as usize;
                let next_level = level + 1;
                let alpha = lod - level as f32; // fractional part

                // Sample current level
                let c0 = self.get_pixel_bilinear_level(u, v, level);

                if next_level > self.mips.len() {
                    return c0;
                }

                let c1 = self.get_pixel_bilinear_level(u, v, next_level);

                // Blend c0 and c1
                let w = (alpha * 256.0) as u32;
                let inv_w = 256 - w;
                blend_swar(c0, c1, w, inv_w)
            }
        }
    }

    fn get_dims(&self, level: usize) -> (u32, u32) {
        if level == 0 {
            (self.width, self.height)
        } else {
            let shift = level as u32;
            let w = (self.width >> shift).max(1);
            let h = (self.height >> shift).max(1);
            (w, h)
        }
    }

    fn get_pixel_bilinear_level(&self, u: f32, v: f32, level: usize) -> u32 {
        let (w, h) = self.get_dims(level);
        let u_tex = u * w as f32;
        let v_tex = v * h as f32;

        let u_fixed = (u_tex * 256.0) as i32;
        let v_fixed = (v_tex * 256.0) as i32;

        let pixels = if level == 0 {
            &self.pixels
        } else {
            if level - 1 < self.mips.len() {
                &self.mips[level - 1]
            } else {
                return 0xFF00_0000;
            }
        };

        Self::get_pixel_bilinear_fixed_impl(pixels, w, h, u_fixed, v_fixed)
    }

    #[must_use]
    pub fn get_pixel_texel(&self, x: i32, y: i32) -> u32 {
        let x = x.clamp(0, self.width as i32 - 1) as usize;
        let y = y.clamp(0, self.height as i32 - 1) as usize;
        unsafe { *self.pixels.get_unchecked(y * self.width as usize + x) }
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
        let u_fixed = (u_tex * 256.0) as i32;
        let v_fixed = (v_tex * 256.0) as i32;
        self.get_pixel_bilinear_fixed(u_fixed, v_fixed)
    }

    /// Sample texture using bilinear interpolation with 24.8 fixed point texel coordinates
    #[inline]
    #[must_use]
    pub fn get_pixel_bilinear_fixed(&self, u_fixed: i32, v_fixed: i32) -> u32 {
        Self::get_pixel_bilinear_fixed_impl(&self.pixels, self.width, self.height, u_fixed, v_fixed)
    }

    #[inline(always)]
    fn get_pixel_bilinear_fixed_impl(pixels: &[u32], width: u32, height: u32, u_fixed: i32, v_fixed: i32) -> u32 {
        let u_img_fixed = u_fixed - 128;
        let v_img_fixed = v_fixed - 128;

        let wx = (u_img_fixed & 0xFF) as u32;
        let wy = (v_img_fixed & 0xFF) as u32;
        let inv_wx = 256 - wx;
        let inv_wy = 256 - wy;

        let x0_raw = u_img_fixed >> 8;
        let y0_raw = v_img_fixed >> 8;

        let (c00, c10, c01, c11) = Self::fetch_bilinear_neighbors_impl(pixels, width, height, x0_raw, y0_raw);

        let top = blend_swar(c00, c10, wx, inv_wx);
        let bottom = blend_swar(c01, c11, wx, inv_wx);
        let final_color = blend_swar(top, bottom, wy, inv_wy);

        final_color | 0xFF00_0000
    }

    #[inline(always)]
    fn fetch_bilinear_neighbors_impl(pixels: &[u32], width: u32, height: u32, x0_raw: i32, y0_raw: i32) -> (u32, u32, u32, u32) {
        let w_i32 = width as i32 - 1;
        let h_i32 = height as i32 - 1;

        if x0_raw >= 0 && x0_raw < w_i32 && y0_raw >= 0 && y0_raw < h_i32 {
            let x0 = x0_raw as usize;
            let y0 = y0_raw as usize;
            let width_usize = width as usize;
            let row0 = y0 * width_usize;
            let row1 = row0 + width_usize;

            unsafe {
                (
                    *pixels.get_unchecked(row0 + x0),
                    *pixels.get_unchecked(row0 + x0 + 1),
                    *pixels.get_unchecked(row1 + x0),
                    *pixels.get_unchecked(row1 + x0 + 1),
                )
            }
        } else {
            let x0 = x0_raw.clamp(0, w_i32) as usize;
            let y0 = y0_raw.clamp(0, h_i32) as usize;
            let x1 = (x0_raw + 1).clamp(0, w_i32) as usize;
            let y1 = (y0_raw + 1).clamp(0, h_i32) as usize;

            let width_usize = width as usize;
            let row0 = y0 * width_usize;
            let row1 = y1 * width_usize;

            unsafe {
                (
                    *pixels.get_unchecked(row0 + x0),
                    *pixels.get_unchecked(row0 + x1),
                    *pixels.get_unchecked(row1 + x0),
                    *pixels.get_unchecked(row1 + x1),
                )
            }
        }
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
