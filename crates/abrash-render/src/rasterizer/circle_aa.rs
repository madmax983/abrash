use abrash_core::framebuffer::Framebuffer;
use abrash_core::color::Color;

/// Draw an anti-aliased circle outline using Xiaolin Wu's line algorithm principles applied to a circle.
pub fn draw_circle_aa(fb: &mut Framebuffer, xc: i32, yc: i32, radius: i32, base_color: u32) {
    if radius <= 0 {
        return;
    }

    // Fast path bounds check
    let min_x = i64::from(xc) - i64::from(radius) - 1;
    let max_x = i64::from(xc) + i64::from(radius) + 1;
    let min_y = i64::from(yc) - i64::from(radius) - 1;
    let max_y = i64::from(yc) + i64::from(radius) + 1;

    if min_x >= i64::from(fb.width()) || max_x < 0 || min_y >= i64::from(fb.height()) || max_y < 0 {
        return; // Fully offscreen
    }

    if min_x >= 0 && max_x < i64::from(fb.width()) && min_y >= 0 && max_y < i64::from(fb.height()) {
        draw_circle_aa_fast(fb, xc, yc, radius, base_color);
        return;
    }

    let a = (base_color >> 24) & 0xFF;
    let r = (base_color >> 16) & 0xFF;
    let g = (base_color >> 8) & 0xFF;
    let b = base_color & 0xFF;

    let mut x = radius as f32;

    // Draw the 4 cardinal points (which are perfectly aligned)
    plot_4(fb, xc, yc, radius, base_color);

    let r_f32 = radius as f32;
    let r_sq = r_f32 * r_f32;

    let mut y_int = 1;
    while (y_int as f32) < x {
        let y = y_int as f32;

        // Newton-Raphson refinement or just f32 sqrt since it maps to CPU intrinsic
        x = (r_sq - y * y).sqrt();

        let x_int = x.floor() as i32;
        let x_frac = x - x_int as f32;

        let alpha_outer = (x_frac * 255.0) as u32;
        let alpha_inner = 255 - alpha_outer;

        // Multiply by base alpha to support translucent colors
        // Fast right shift equivalent to / 255 with slight precision loss
        let final_alpha_outer = (alpha_outer * a) >> 8;
        let final_alpha_inner = (alpha_inner * a) >> 8;

        let color_outer = (final_alpha_outer << 24) | (r << 16) | (g << 8) | b;
        let color_inner = (final_alpha_inner << 24) | (r << 16) | (g << 8) | b;

        plot_8(fb, xc, yc, x_int, y_int, color_inner);
        plot_8(fb, xc, yc, x_int + 1, y_int, color_outer);

        y_int += 1;
    }
}

#[inline(always)]
fn draw_circle_aa_fast(fb: &mut Framebuffer, xc: i32, yc: i32, radius: i32, base_color: u32) {
    if radius <= 0 {
        return;
    }

    let a = (base_color >> 24) & 0xFF;
    let r = (base_color >> 16) & 0xFF;
    let g = (base_color >> 8) & 0xFF;
    let b = base_color & 0xFF;

    let mut x = radius as f32;

    // Draw the 4 cardinal points (which are perfectly aligned)
    plot_4_fast(fb, xc, yc, radius, base_color);

    let r_f32 = radius as f32;
    let r_sq = r_f32 * r_f32;

    let mut y_int = 1;
    while (y_int as f32) < x {
        let y = y_int as f32;

        // Exact x = sqrt(r^2 - y^2)
        x = (r_sq - y * y).sqrt();

        let x_int = x.floor() as i32;
        let x_frac = x - x_int as f32;

        let alpha_outer = (x_frac * 255.0) as u32;
        let alpha_inner = 255 - alpha_outer;

        // Multiply by base alpha to support translucent colors
        // Fast right shift equivalent to / 255 with slight precision loss
        let final_alpha_outer = (alpha_outer * a) >> 8;
        let final_alpha_inner = (alpha_inner * a) >> 8;

        let color_outer = (final_alpha_outer << 24) | (r << 16) | (g << 8) | b;
        let color_inner = (final_alpha_inner << 24) | (r << 16) | (g << 8) | b;

        plot_8_fast(fb, xc, yc, x_int, y_int, color_inner);
        plot_8_fast(fb, xc, yc, x_int + 1, y_int, color_outer);

        y_int += 1;
    }
}

#[inline(always)]
fn blend_pixel(fb: &mut Framebuffer, x: i32, y: i32, color: u32) {
    let alpha = (color >> 24) & 0xFF;
    if alpha == 0 {
        return;
    }

    if alpha == 255 {
        fb.set_pixel(x, y, color);
        return;
    }

    if let Some(bg_color) = fb.get_pixel(x, y) {
        let c_src = Color::from_argb_u32(color);
        let c_dst = Color::from_argb_u32(bg_color);
        let blended = Color::blend_over(c_src, c_dst).to_argb_u32();
        fb.set_pixel(x, y, blended);
    }
}

#[inline(always)]
fn blend_pixel_fast(fb: &mut Framebuffer, x: usize, y: usize, color: u32) {
    let alpha = (color >> 24) & 0xFF;
    if alpha == 0 {
        return;
    }

    unsafe {
        if alpha == 255 {
            fb.set_pixel_unchecked(x, y, color);
            return;
        }

        let bg_color = fb.get_pixel_unchecked(x, y);
        // Fast integer blending to avoid float conversions in hot loop
        let alpha = color >> 24;
        let inv_alpha = 255 - alpha;

        let sr = (color >> 16) & 0xFF;
        let sg = (color >> 8) & 0xFF;
        let sb = color & 0xFF;

        let dr = (bg_color >> 16) & 0xFF;
        let dg = (bg_color >> 8) & 0xFF;
        let db = bg_color & 0xFF;

        let out_r = ((sr * alpha) + (dr * inv_alpha)) >> 8;
        let out_g = ((sg * alpha) + (dg * inv_alpha)) >> 8;
        let out_b = ((sb * alpha) + (db * inv_alpha)) >> 8;
        // Approximation of alpha blending over an opaque background (dest alpha is 255)
        let blended = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;

        fb.set_pixel_unchecked(x, y, blended);
    }
}

#[inline(always)]
fn plot_4(fb: &mut Framebuffer, xc: i32, yc: i32, radius: i32, color: u32) {
    blend_pixel(fb, xc, yc + radius, color);
    blend_pixel(fb, xc, yc - radius, color);
    blend_pixel(fb, xc + radius, yc, color);
    blend_pixel(fb, xc - radius, yc, color);
}

#[inline(always)]
fn plot_4_fast(fb: &mut Framebuffer, xc: i32, yc: i32, radius: i32, color: u32) {
    blend_pixel_fast(fb, xc as usize, (yc + radius) as usize, color);
    blend_pixel_fast(fb, xc as usize, (yc - radius) as usize, color);
    blend_pixel_fast(fb, (xc + radius) as usize, yc as usize, color);
    blend_pixel_fast(fb, (xc - radius) as usize, yc as usize, color);
}

#[inline(always)]
fn plot_8(fb: &mut Framebuffer, xc: i32, yc: i32, x: i32, y: i32, color: u32) {
    blend_pixel(fb, xc + x, yc + y, color);
    blend_pixel(fb, xc - x, yc + y, color);
    blend_pixel(fb, xc + x, yc - y, color);
    blend_pixel(fb, xc - x, yc - y, color);
    blend_pixel(fb, xc + y, yc + x, color);
    blend_pixel(fb, xc - y, yc + x, color);
    blend_pixel(fb, xc + y, yc - x, color);
    blend_pixel(fb, xc - y, yc - x, color);
}

#[inline(always)]
fn plot_8_fast(fb: &mut Framebuffer, xc: i32, yc: i32, x: i32, y: i32, color: u32) {
    blend_pixel_fast(fb, (xc + x) as usize, (yc + y) as usize, color);
    blend_pixel_fast(fb, (xc - x) as usize, (yc + y) as usize, color);
    blend_pixel_fast(fb, (xc + x) as usize, (yc - y) as usize, color);
    blend_pixel_fast(fb, (xc - x) as usize, (yc - y) as usize, color);
    blend_pixel_fast(fb, (xc + y) as usize, (yc + x) as usize, color);
    blend_pixel_fast(fb, (xc - y) as usize, (yc + x) as usize, color);
    blend_pixel_fast(fb, (xc + y) as usize, (yc - x) as usize, color);
    blend_pixel_fast(fb, (xc - y) as usize, (yc - x) as usize, color);
}
