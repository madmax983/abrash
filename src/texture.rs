//! Texture mapping support.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterMode {
    Nearest,
    Bilinear,
}

/// A simple 2D texture
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
    pub filter_mode: FilterMode,
}

impl Texture {
    pub fn new(width: u32, height: u32) -> Self {
        assert!(
            width > 0 && height > 0,
            "Texture dimensions must be positive"
        );
        Self {
            width,
            height,
            pixels: vec![0xFF000000; (width * height) as usize],
            filter_mode: FilterMode::Nearest,
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = color;
        }
    }

    /// Sample texture using interpolation mode
    /// u, v are in range [0.0, 1.0]
    #[inline]
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
    pub fn get_pixel_bilinear(&self, u: f32, v: f32) -> u32 {
        let w = self.width as f32;
        let h = self.height as f32;
        self.get_pixel_bilinear_texel(u * w, v * h)
    }

    /// Sample texture using bilinear interpolation with texel coordinates
    #[inline]
    pub fn get_pixel_bilinear_texel(&self, u_tex: f32, v_tex: f32) -> u32 {
        // Convert to 24.8 fixed point
        // 0.5 in 24.8 is 128
        let u_fixed = (u_tex * 256.0) as i32;
        let v_fixed = (v_tex * 256.0) as i32;

        let u_img_fixed = u_fixed - 128;
        let v_img_fixed = v_fixed - 128;

        // Weights (0..256)
        let wx = (u_img_fixed & 0xFF) as u32;
        let wy = (v_img_fixed & 0xFF) as u32;
        let inv_wx = 256 - wx;
        let inv_wy = 256 - wy;

        // Coordinates
        let w_i32 = self.width as i32 - 1;
        let h_i32 = self.height as i32 - 1;

        // Arithmetic shift preserves sign (floor behavior for negative numbers)
        let x0_raw = u_img_fixed >> 8;
        let y0_raw = v_img_fixed >> 8;

        let x0 = x0_raw.clamp(0, w_i32) as usize;
        let y0 = y0_raw.clamp(0, h_i32) as usize;
        let x1 = (x0_raw + 1).clamp(0, w_i32) as usize;
        let y1 = (y0_raw + 1).clamp(0, h_i32) as usize;

        let width_usize = self.width as usize;
        let row0 = y0 * width_usize;
        let row1 = y1 * width_usize;

        // SAFETY: We clamped coordinates to valid ranges [0, width-1] / [0, height-1]
        let (c00, c10, c01, c11) = unsafe {
            (
                *self.pixels.get_unchecked(row0 + x0),
                *self.pixels.get_unchecked(row0 + x1),
                *self.pixels.get_unchecked(row1 + x0),
                *self.pixels.get_unchecked(row1 + x1),
            )
        };

        // Function to blend two colors with weight w using SWAR (SIMD Within A Register)
        // Blends R/B and A/G in parallel
        let blend = |c0: u32, c1: u32, w: u32, inv_w: u32| -> u32 {
            let rb0 = c0 & 0x00FF00FF;
            let ag0 = (c0 >> 8) & 0x00FF00FF;
            let rb1 = c1 & 0x00FF00FF;
            let ag1 = (c1 >> 8) & 0x00FF00FF;

            let rb = ((rb0 * inv_w + rb1 * w) >> 8) & 0x00FF00FF;
            let ag = ((ag0 * inv_w + ag1 * w) >> 8) & 0x00FF00FF;

            rb | (ag << 8)
        };

        let top = blend(c00, c10, wx, inv_wx);
        let bottom = blend(c01, c11, wx, inv_wx);
        let final_color = blend(top, bottom, wy, inv_wy);

        // Ensure alpha is 0xFF
        final_color | 0xFF000000
    }

    /// Sample texture using texel coordinates
    #[inline]
    pub fn get_pixel_texel(&self, x: i32, y: i32) -> u32 {
        let x = x.clamp(0, self.width as i32 - 1) as usize;
        let y = y.clamp(0, self.height as i32 - 1) as usize;
        unsafe { *self.pixels.get_unchecked(y * self.width as usize + x) }
    }

    /// Create a checkerboard texture
    pub fn checkered(width: u32, height: u32, c1: u32, c2: u32) -> Self {
        let mut tex = Self::new(width, height);
        // Scale checks based on size, defaulting to 8x8 blocks
        let block_w = (width / 8).max(1);
        let block_h = (height / 8).max(1);

        for y in 0..height {
            for x in 0..width {
                let check = ((x / block_w) + (y / block_h)) & 1 == 0;
                tex.set_pixel(x, y, if check { c1 } else { c2 });
            }
        }
        tex
    }
}
