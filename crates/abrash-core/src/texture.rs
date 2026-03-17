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
    /// Trilinear interpolation. Smoother with mipmaps, best quality but slower.
    Trilinear,
}

/// Helper for bilinear interpolation blending using SWAR (SIMD Within A Register).
///
/// **SWAR** is a technique to perform parallel operations on data packed into general-purpose registers,
/// avoiding the need for specialized SIMD instructions (like AVX or NEON).
///
/// This function treats a 32-bit integer as two 16-bit slots (or four 8-bit slots with gaps)
/// to blend two color channels simultaneously.
///
/// *   `c0`, `c1`: Packed color channels (e.g., Red/Blue or Green/Alpha).
/// *   `w`, `inv_w`: Weight and inverse weight (sum must be 256 for 8-bit precision).
#[inline(always)]
#[must_use]
pub const fn blend_swar(c0: u32, c1: u32, w: u32, inv_w: u32) -> u32 {
    let rb0 = c0 & 0x00FF_00FF;
    let ag0 = (c0 >> 8) & 0x00FF_00FF;
    let rb1 = c1 & 0x00FF_00FF;
    let ag1 = (c1 >> 8) & 0x00FF_00FF;

    let rb = ((rb0 * inv_w + rb1 * w) >> 8) & 0x00FF_00FF;
    let ag = ((ag0 * inv_w + ag1 * w) >> 8) & 0x00FF_00FF;

    rb | (ag << 8)
}

/// Helper for 4-way bilinear blending using SWAR accumulation.
///
/// This is more efficient than calling `blend_swar` 3 times because it minimizes bitwise masking and shifting.
/// It unpacks the 4 colors once, accumulates weighted contributions, and then packs the result.
///
/// *   `c00`, `c10`, `c01`, `c11`: Neighbors (Top-Left, Top-Right, Bottom-Left, Bottom-Right).
/// *   `wx`, `wy`: Horizontal and vertical weights (0..256).
#[inline(always)]
#[must_use]
pub const fn blend_four_way(c00: u32, c10: u32, c01: u32, c11: u32, wx: u32, wy: u32) -> u32 {
    let inv_wx = 256 - wx;
    let inv_wy = 256 - wy;

    // Weights for each corner (u64 to match accumulator width)
    let w00 = (inv_wx * inv_wy) as u64;
    let w10 = (wx * inv_wy) as u64;
    let w01 = (inv_wx * wy) as u64;
    let w11 = (wx * wy) as u64;

    // Unpack and accumulate
    // Use u64 to prevent overflow when multiplying 8-bit components by 16-bit weights
    // (max result ~16.7 million, which fits in 24 bits, but packed channels overlap in u32)
    let mut acc_rb = (c00 as u64 & 0x00FF_00FF) * w00;
    acc_rb += (c10 as u64 & 0x00FF_00FF) * w10;
    acc_rb += (c01 as u64 & 0x00FF_00FF) * w01;
    acc_rb += (c11 as u64 & 0x00FF_00FF) * w11;

    let mut acc_ag = ((c00 as u64 >> 8) & 0x00FF_00FF) * w00;
    acc_ag += ((c10 as u64 >> 8) & 0x00FF_00FF) * w10;
    acc_ag += ((c01 as u64 >> 8) & 0x00FF_00FF) * w01;
    acc_ag += ((c11 as u64 >> 8) & 0x00FF_00FF) * w11;

    // Pack
    let rb = (acc_rb >> 16) as u32 & 0x00FF_00FF;
    let ag = (acc_ag >> 16) as u32 & 0x00FF_00FF;

    rb | (ag << 8)
}

/// Helper to average 4 colors (simple box filter)
const fn average_4_colors(c00: u32, c10: u32, c01: u32, c11: u32) -> u32 {
    let r =
        (((c00 >> 16) & 0xFF) + ((c10 >> 16) & 0xFF) + ((c01 >> 16) & 0xFF) + ((c11 >> 16) & 0xFF))
            / 4;
    let g =
        (((c00 >> 8) & 0xFF) + ((c10 >> 8) & 0xFF) + ((c01 >> 8) & 0xFF) + ((c11 >> 8) & 0xFF)) / 4;
    let b = ((c00 & 0xFF) + (c10 & 0xFF) + (c01 & 0xFF) + (c11 & 0xFF)) / 4;

    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// A simple 2D texture.
#[derive(Clone)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    /// Shift amount for power-of-two textures (log2(width)).
    /// Used to replace multiplication with shifting for index calculation.
    /// Value is `0xFF` if width is not a power of two.
    pub width_shift: u8,
    pub pixels: Vec<u32>,
    /// Mipmap levels. Level 0 is implicit in `pixels`. `mips[0]` is Level 1, etc.
    pub mips: Vec<Vec<u32>>,
    /// The filtering mode to use when sampling the texture.
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
    /// use abrash_core::texture::Texture;
    ///
    /// let texture = Texture::new(256, 256).unwrap();
    /// assert_eq!(texture.width(), 256);
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
            .filter(|&s| i32::try_from(s).is_ok())
            .ok_or("Texture size overflow (max i32::MAX pixels)")? as usize;

        let width_shift = if width.is_power_of_two() {
            width.trailing_zeros() as u8
        } else {
            0xFF
        };

        Ok(Self {
            width,
            height,
            width_shift,
            pixels: vec![0xFF00_0000; size],
            mips: Vec::new(),
            filter_mode: FilterMode::Nearest,
        })
    }

    /// Calculate the byte offset for the start of a row.
    ///
    /// Uses bitwise shift if `width` is a power of two, otherwise falls back to multiplication.
    #[inline(always)]
    #[must_use]
    pub const fn row_offset(&self, y: usize) -> usize {
        if self.width_shift < 32 {
            y << self.width_shift
        } else {
            y * (self.width as usize)
        }
    }

    /// Sets a pixel color at the specified coordinates.
    ///
    /// Coordinates are 0-based.
    /// Does nothing if coordinates are out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::texture::Texture;
    ///
    /// let mut tex = Texture::new(4, 4).unwrap();
    /// // Set pixel at (2, 2) to Red
    /// tex.set_pixel(2, 2, 0xFFFF0000);
    /// ```
    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            let idx = self.row_offset(y as usize) + (x as usize);
            self.pixels[idx] = color;
        }
    }

    /// Generates mipmaps for the texture.
    /// Should be called after modifying pixels if Trilinear filtering is used.
    ///
    /// # Algorithm
    ///
    /// Uses a **Box Filter** (Simple Average) to downsample the texture.
    /// Each pixel in mip level `L+1` is the arithmetic mean of the corresponding
    /// 2x2 block of pixels in level `L`.
    ///
    /// `$$ P_{L+1}(x,y) = \frac{P_L(2x,2y) + P_L(2x+1,2y) + P_L(2x,2y+1) + P_L(2x+1,2y+1)}{4} $$`
    ///
    /// # Panics
    ///
    /// Panics if internal logic fails to retrieve previous mip level.
    pub fn generate_mipmaps(&mut self) {
        let mut width = self.width;
        let mut height = self.height;
        // Start from base level
        // We can't hold a reference to `self.pixels` while pushing to `self.mips`
        // So we will reconstruct the previous level based on index

        self.mips.clear();

        while width > 1 || height > 1 {
            let next_width = (width / 2).max(1);
            let next_height = (height / 2).max(1);
            let size = (next_width * next_height) as usize;
            let mut next_pixels = Vec::with_capacity(size);

            // Get previous level pixels
            let prev_pixels = if self.mips.is_empty() {
                &self.pixels
            } else {
                self.mips.last().unwrap()
            };

            for y in 0..next_height {
                for x in 0..next_width {
                    // Box filter: average 2x2 block from previous level
                    let src_x = x * 2;
                    let src_y = y * 2;

                    // Helper to get pixel safely
                    let get = |px: u32, py: u32| -> u32 {
                        let px = px.min(width - 1);
                        let py = py.min(height - 1);
                        prev_pixels[(py * width + px) as usize]
                    };

                    let p00 = get(src_x, src_y);
                    let p10 = get(src_x + 1, src_y);
                    let p01 = get(src_x, src_y + 1);
                    let p11 = get(src_x + 1, src_y + 1);

                    let avg = average_4_colors(p00, p10, p01, p11);
                    next_pixels.push(avg);
                }
            }

            self.mips.push(next_pixels);
            width = next_width;
            height = next_height;
        }
    }

    /// Sample texture using interpolation mode
    /// u, v are in range [0.0, 1.0]
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::texture::Texture;
    ///
    /// let mut tex = Texture::new(2, 2).unwrap();
    /// tex.set_pixel(0, 0, 0xFFFF_FFFF);
    ///
    /// // Sample center of top-left pixel
    /// let color = tex.get_pixel(0.25, 0.25);
    /// assert_eq!(color, 0xFFFF_FFFF);
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
            FilterMode::Bilinear | FilterMode::Trilinear => self.get_pixel_bilinear(u, v),
        }
    }

    /// Sample texture using trilinear interpolation with given LOD
    #[must_use]
    pub fn get_pixel_trilinear(&self, u: f32, v: f32, lod: f32) -> u32 {
        if lod <= 0.0 || self.mips.is_empty() {
            return self.get_pixel_bilinear(u, v);
        }

        let max_level = self.mips.len() as f32;
        if lod >= max_level {
            // Sample max level
            return self.sample_mip(u, v, self.mips.len() - 1);
        }

        let level = lod.floor();
        let frac = lod - level;
        let level_idx = level as usize;

        let c0 = if level_idx == 0 {
            self.get_pixel_bilinear(u, v)
        } else {
            self.sample_mip(u, v, level_idx - 1)
        };

        let c1 = self.sample_mip(u, v, level_idx);

        // Blend c0 and c1
        // We can reuse blend_swar if we convert frac to integer weight
        let weight = (frac * 256.0) as u32;
        let inv_weight = 256 - weight;

        blend_swar(c0, c1, weight, inv_weight)
    }

    /// Sample texture using trilinear interpolation with given LOD and 16.16 fixed point UVs
    #[inline]
    #[must_use]
    pub fn get_pixel_trilinear_fixed(&self, u_fix: i32, v_fix: i32, lod: f32) -> u32 {
        if lod <= 0.0 || self.mips.is_empty() {
            // Level 0 (Base). 16.16 >> 8 -> 24.8
            return self.get_pixel_bilinear_fixed(u_fix >> 8, v_fix >> 8);
        }

        let max_level = self.mips.len() as f32;
        if lod >= max_level {
            // Max level.
            let mip_idx = self.mips.len() - 1;
            // Level L = mip_idx + 1. Shift = 8 + L = 9 + mip_idx.
            let shift = 9 + mip_idx;
            return self.sample_mip_fixed(u_fix >> shift, v_fix >> shift, mip_idx);
        }

        let level = lod.floor();
        let frac = lod - level;
        let level_idx = level as usize;

        let c0 = if level_idx == 0 {
            self.get_pixel_bilinear_fixed(u_fix >> 8, v_fix >> 8)
        } else {
            let mip_idx = level_idx - 1;
            let shift = 9 + mip_idx;
            self.sample_mip_fixed(u_fix >> shift, v_fix >> shift, mip_idx)
        };

        let c1 = {
            let mip_idx = level_idx;
            let shift = 9 + mip_idx;
            self.sample_mip_fixed(u_fix >> shift, v_fix >> shift, mip_idx)
        };

        // Blend c0 and c1
        let weight = (frac * 256.0) as u32;
        let inv_weight = 256 - weight;

        blend_swar(c0, c1, weight, inv_weight)
    }

    /// Helper to sample a specific mip level
    fn sample_mip(&self, u: f32, v: f32, mip_idx: usize) -> u32 {
        let width = (self.width >> (mip_idx + 1)).max(1);
        let height = (self.height >> (mip_idx + 1)).max(1);

        let w = width as f32;
        let h = height as f32;

        let u_tex = u * w;
        let v_tex = v * h;

        let u_fixed = (u_tex * 256.0) as i32;
        let v_fixed = (v_tex * 256.0) as i32;

        self.sample_mip_fixed(u_fixed, v_fixed, mip_idx)
    }

    /// Helper to sample a specific mip level with fixed point coordinates (24.8)
    #[inline]
    fn sample_mip_fixed(&self, u_fixed: i32, v_fixed: i32, mip_idx: usize) -> u32 {
        let pixels = &self.mips[mip_idx];
        let width = (self.width >> (mip_idx + 1)).max(1);
        let height = (self.height >> (mip_idx + 1)).max(1);

        let u_img_fixed = u_fixed.wrapping_sub(128);
        let v_img_fixed = v_fixed.wrapping_sub(128);

        let wx = (u_img_fixed & 0xFF) as u32;
        let wy = (v_img_fixed & 0xFF) as u32;
        let inv_wx = 256 - wx;
        let inv_wy = 256 - wy;

        let x0_raw = u_img_fixed >> 8;
        let y0_raw = v_img_fixed >> 8;

        let w_i32 = width as i32 - 1;
        let h_i32 = height as i32 - 1;

        // Manual neighbor fetch for mips
        let (c00, c10, c01, c11) = {
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
        };

        let top = blend_swar(c00, c10, wx, inv_wx);
        let bottom = blend_swar(c01, c11, wx, inv_wx);
        blend_swar(top, bottom, wy, inv_wy)
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
        self.get_pixel_bilinear_fixed_no_offset(
            u_fixed.wrapping_sub(128),
            v_fixed.wrapping_sub(128),
        )
    }

    /// Sample texture using bilinear interpolation with 24.8 fixed point texel coordinates
    #[inline]
    #[must_use]
    pub fn get_pixel_bilinear_fixed(&self, u_fixed: i32, v_fixed: i32) -> u32 {
        self.get_pixel_bilinear_fixed_no_offset(
            u_fixed.wrapping_sub(128),
            v_fixed.wrapping_sub(128),
        )
    }

    /// Sample texture using bilinear interpolation with 24.8 fixed point texel coordinates.
    /// Assumes coordinates are already offset by -0.5 (128 units).
    #[inline]
    #[must_use]
    pub fn get_pixel_bilinear_fixed_no_offset(&self, u_img_fixed: i32, v_img_fixed: i32) -> u32 {
        // Weights (0..256)
        let wx = (u_img_fixed & 0xFF) as u32;
        let wy = (v_img_fixed & 0xFF) as u32;

        // Arithmetic shift preserves sign (floor behavior for negative numbers)
        let x0_raw = u_img_fixed >> 8;
        let y0_raw = v_img_fixed >> 8;

        let (c00, c10, c01, c11) = self.fetch_bilinear_neighbors(x0_raw, y0_raw);

        blend_four_way(c00, c10, c01, c11, wx, wy)
    }

    #[inline(always)]
    fn fetch_bilinear_neighbors(&self, x0_raw: i32, y0_raw: i32) -> (u32, u32, u32, u32) {
        let w_i32 = self.width as i32 - 1;
        let h_i32 = self.height as i32 - 1;

        // Optimization: Fast path for interior pixels to avoid 4 clamps
        // w_i32 is width - 1. If x0_raw < w_i32, then x0_raw <= width - 2, so x0_raw + 1 <= width - 1.
        // We use unsigned comparison to combine the non-negative check with the upper bound check.
        if (x0_raw as u32) < (w_i32 as u32) && (y0_raw as u32) < (h_i32 as u32) {
            let x0 = x0_raw as usize;
            let y0 = y0_raw as usize;

            let row0 = self.row_offset(y0);
            let row1 = row0 + (self.width as usize); // y0 + 1 is valid, so row1 is next row

            unsafe {
                // Optimization: Read 2 pixels at a time as u64.
                // This reduces memory instructions and address calculations.
                // We use read_unaligned because x0 might not be 8-byte aligned.
                #[cfg(target_endian = "little")]
                {
                    let ptr = self.pixels.as_ptr();
                    let row0_pair = ptr.add(row0 + x0).cast::<u64>().read_unaligned();
                    let row1_pair = ptr.add(row1 + x0).cast::<u64>().read_unaligned();

                    (
                        row0_pair as u32,
                        (row0_pair >> 32) as u32,
                        row1_pair as u32,
                        (row1_pair >> 32) as u32,
                    )
                }
                #[cfg(not(target_endian = "little"))]
                {
                    (
                        *self.pixels.get_unchecked(row0 + x0),
                        *self.pixels.get_unchecked(row0 + x0 + 1),
                        *self.pixels.get_unchecked(row1 + x0),
                        *self.pixels.get_unchecked(row1 + x0 + 1),
                    )
                }
            }
        } else {
            let x0 = x0_raw.clamp(0, w_i32) as usize;
            let y0 = y0_raw.clamp(0, h_i32) as usize;
            let x1 = (x0_raw + 1).clamp(0, w_i32) as usize;
            let y1 = (y0_raw + 1).clamp(0, h_i32) as usize;

            let row0 = self.row_offset(y0);
            let row1 = self.row_offset(y1);

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

    /// Gets the raw color of a texel at (x, y) without any filtering.
    /// Returns 0 (transparent black) if coordinates are out of bounds.
    #[must_use]
    #[inline]
    pub fn get_pixel_texel(&self, x: i32, y: i32) -> u32 {
        // Optimization: Fast path for in-bounds coordinates.
        // Casting to u32 handles negative checks implicitly (negative i32 becomes large u32).
        if (x as u32) < self.width && (y as u32) < self.height {
            unsafe {
                *self
                    .pixels
                    .get_unchecked(self.row_offset(y as usize) + (x as usize))
            }
        } else {
            let x = x.clamp(0, self.width as i32 - 1) as usize;
            let y = y.clamp(0, self.height as i32 - 1) as usize;
            unsafe { *self.pixels.get_unchecked(self.row_offset(y) + x) }
        }
    }

    /// Returns the width of the texture.
    #[inline]
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height of the texture.
    #[inline]
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns a reference to the pixel buffer.
    #[inline]
    #[must_use]
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    /// Returns a mutable reference to the pixel buffer.
    #[inline]
    #[must_use]
    pub fn pixels_mut(&mut self) -> &mut [u32] {
        &mut self.pixels
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
        tex.generate_mipmaps();
        Ok(tex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_pixel_trilinear_fixed_interpolation() {
        // 4x4 texture with checkerboard
        let mut tex = Texture::new(4, 4).unwrap();
        // Level 0 (4x4): Checkerboard
        // (0,0) Black (00), (1,0) White (FF)
        // (0,1) White (FF), (1,1) Black (00)
        for y in 0..4 {
            for x in 0..4 {
                let color = if (x + y) % 2 == 0 {
                    0xFF00_0000
                } else {
                    0xFFFF_FFFF
                };
                tex.set_pixel(x, y, color);
            }
        }
        tex.generate_mipmaps();

        // Level 1 (2x2):
        // Each pixel is average of 2x2 block from Level 0.
        // Block (0,0) to (1,1): Black, White, White, Black. Avg: 127 (0x7F).
        // So Level 1 should be all grey (0xFF7F7F7F).

        // Test at (0.5, 0.5) texel coordinates (center of top-left pixel of Level 0).
        // Level 0 value: Black (0x00).
        // Level 1 value: Grey (0x7F).
        // LOD 0.5 -> Average of Level 0 and Level 1 -> (0 + 127)/2 = 63 (0x3F).

        let u_fix = 32768; // 0.5 * 65536
        let v_fix = 32768;
        let lod = 0.5;

        let pixel = tex.get_pixel_trilinear_fixed(u_fix, v_fix, lod);
        let r = (pixel >> 16) & 0xFF;

        // Allow some tolerance for integer arithmetic
        assert!((60..=66).contains(&r), "Expected ~63 (0x3F), got {r}");
    }

    #[test]
    fn sample_mip_fixed_level_selection() {
        let mut tex = Texture::new(4, 4).unwrap();
        // Fill base level with Black
        for i in 0..16 {
            tex.pixels[i] = 0xFF00_0000;
        }
        tex.generate_mipmaps();

        // Manually set Level 1 (2x2) to White
        // mips[0] is Level 1.
        if let Some(l1) = tex.mips.get_mut(0) {
            for p in l1.iter_mut() {
                *p = 0xFFFF_FFFF;
            }
        }

        // Test sampling Level 1 directly via LOD=1.0
        // u=0.5, v=0.5.
        // Level 0 (Base): Black.
        // Level 1 (mips[0]): White.
        // Result should be White.

        let u_fix = 32768;
        let v_fix = 32768;
        let pixel = tex.get_pixel_trilinear_fixed(u_fix, v_fix, 1.0);

        assert_eq!(
            pixel, 0xFFFF_FFFF,
            "LOD 1.0 should sample from Level 1 (White)"
        );

        // Test LOD=0.0 -> Black
        let pixel_l0 = tex.get_pixel_trilinear_fixed(u_fix, v_fix, 0.0);
        assert_eq!(
            pixel_l0, 0xFF00_0000,
            "LOD 0.0 should sample from Level 0 (Black)"
        );
    }
}
