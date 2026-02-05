import sys

with open('src/rasterizer.rs', 'r') as f:
    content = f.read()

# 1. Add get_pixel_bilinear_fixed and update get_pixel_bilinear_texel
old_texel = """    /// Sample texture using bilinear interpolation with texel coordinates
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
    }"""

new_texel = """    /// Sample texture using bilinear interpolation with texel coordinates
    #[inline]
    pub fn get_pixel_bilinear_texel(&self, u_tex: f32, v_tex: f32) -> u32 {
        // Convert to 24.8 fixed point
        // 0.5 in 24.8 is 128
        let u_fixed = (u_tex * 256.0) as i32;
        let v_fixed = (v_tex * 256.0) as i32;
        self.get_pixel_bilinear_fixed(u_fixed, v_fixed)
    }

    /// Sample texture using bilinear interpolation with 24.8 fixed point texel coordinates
    #[inline]
    pub fn get_pixel_bilinear_fixed(&self, u_fixed: i32, v_fixed: i32) -> u32 {
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
    }"""

if old_texel in content:
    content = content.replace(old_texel, new_texel)
else:
    print("Could not find get_pixel_bilinear_texel block")
    # sys.exit(1)

# 2. Optimize draw_scanline_textured_perspective
old_loop = """            FilterMode::Bilinear => {
                let mut u_tex = u_tex_start;
                let mut v_tex = v_tex_start;
                for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                    if z < *depth_val {
                        *depth_val = z;
                        *pixel = texture.get_pixel_bilinear_texel(u_tex, v_tex);
                    }
                    z += gradients.dz_dx;
                    u_tex += du_tex_step;
                    v_tex += dv_tex_step;
                }
            }"""

new_loop = """            FilterMode::Bilinear => {
                // Fixed point optimization for Bilinear
                // Use 16.16 for accumulation to maintain precision, then downshift to 24.8 for sampling
                let mut u_fix = (u_tex_start * 65536.0) as i32;
                let mut v_fix = (v_tex_start * 65536.0) as i32;
                let du_fix = (du_tex_step * 65536.0) as i32;
                let dv_fix = (dv_tex_step * 65536.0) as i32;

                for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
                    if z < *depth_val {
                        *depth_val = z;
                        // Convert 16.16 to 24.8 (x >> 8)
                        *pixel = texture.get_pixel_bilinear_fixed(u_fix >> 8, v_fix >> 8);
                    }
                    z += gradients.dz_dx;
                    u_fix = u_fix.wrapping_add(du_fix);
                    v_fix = v_fix.wrapping_add(dv_fix);
                }
            }"""

if old_loop in content:
    content = content.replace(old_loop, new_loop)
else:
    print("Could not find FilterMode::Bilinear block")
    # sys.exit(1)

with open('src/rasterizer.rs', 'w') as f:
    f.write(content)

print("Successfully applied changes.")
