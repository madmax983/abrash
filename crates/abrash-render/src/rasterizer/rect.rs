//! # 2D Rectangles 📏
//!
//! Provides routines for drawing and filling standard and rounded 2D rectangles
//! directly into the framebuffer. These functions are often used for UI overlays
//! or debug visualizations.
//!
//! ## Examples
//!
//! ```
//! use abrash_core::framebuffer::Framebuffer;
//! use abrash_render::rasterizer::rect::draw_rect;
//!
//! let mut fb = Framebuffer::new(100, 100).unwrap();
//! draw_rect(&mut fb, 10, 10, 20, 20, 0x00FF_FFFFFF);
//! ```

use abrash_core::framebuffer::Framebuffer;
use abrash_core::ivec::IVec2;

/// ⚡ Bolt: Fast horizontal line fill that avoids per-pixel bounds checks in the inner loop.
#[inline(always)]
fn draw_horizontal_line(fb: &mut Framebuffer, x0: i32, x1: i32, y: i32, color: u32) {
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

    let x_start = x0.max(0);
    let x_end = x1.min(fb.width() as i32 - 1);

    if x_start > x_end {
        return;
    }

    let w = fb.width() as usize;
    let start_idx = y as usize * w + x_start as usize;
    let end_idx = y as usize * w + x_end as usize;

    // ⚡ Bolt: Elide bounds check with get_unchecked_mut in Fill
    unsafe {
        fb.as_mut_slice()
            .get_unchecked_mut(start_idx..=end_idx)
            .fill(color);
    }
}

/// ⚡ Bolt: Direct horizontal line fill skipping boundary checking completely
/// Warning: Only call when it is guaranteed that the span is on screen.
#[inline(always)]
fn draw_horizontal_line_unchecked(fb: &mut Framebuffer, x0: i32, x1: i32, y: i32, color: u32) {
    let w = fb.width() as usize;
    let start_idx = y as usize * w + x0 as usize;
    let end_idx = y as usize * w + x1 as usize;

    if start_idx > end_idx || end_idx >= fb.as_mut_slice().len() {
        return;
    }

    // ⚡ Bolt: Elide bounds check with get_unchecked_mut in Fill
    unsafe {
        fb.as_mut_slice()
            .get_unchecked_mut(start_idx..=end_idx)
            .fill(color);
    }
}

/// ⚡ Bolt: Fast vertical line drawing skipping generalized Bresenham logic.
#[inline(always)]
fn draw_vertical_line(fb: &mut Framebuffer, x: i32, y0: i32, y1: i32, color: u32) {
    if x < 0 || x >= fb.width() as i32 {
        return;
    }

    let y_start = y0.max(0);
    let y_end = y1.min(fb.height() as i32 - 1);

    if y_start > y_end {
        return;
    }

    let w = fb.width() as usize;
    let mut idx = y_start as usize * w + x as usize;
    let slice = fb.as_mut_slice();

    // ⚡ Bolt: Step exactly by framebuffer width
    for _ in y_start..=y_end {
        if idx < slice.len() {
            unsafe {
                *slice.get_unchecked_mut(idx) = color;
            }
        }
        idx += w;
    }
}

/// ⚡ Bolt: Direct vertical line drawing skipping boundary checks.
/// Warning: Only call when it is guaranteed that the span is on screen.
#[inline(always)]
fn draw_vertical_line_unchecked(fb: &mut Framebuffer, x: i32, y0: i32, y1: i32, color: u32) {
    let w = fb.width() as usize;
    let mut idx = y0 as usize * w + x as usize;
    let slice = fb.as_mut_slice();

    if idx >= slice.len() || (y1 as usize * w + x as usize) >= slice.len() {
        return;
    }

    // ⚡ Bolt: Step exactly by framebuffer width, completely elide bounds checks
    for _ in y0..=y1 {
        unsafe {
            *slice.get_unchecked_mut(idx) = color;
        }
        idx += w;
    }
}

/// Draws the outline of a 2D rectangle.
///
/// This draws lines directly onto the framebuffer, skipping the complex 3D rasterization pipeline.
/// It is incredibly useful for rendering fast UI overlays, selection boxes, or debugging boundaries
/// on top of an already rendered 3D scene without needing to push vertices through the camera transform.
///
/// ## Parameters
///
/// * `fb` - The target framebuffer.
/// * `x` - The left X coordinate.
/// * `y` - The top Y coordinate.
/// * `width` - The width of the rectangle.
/// * `height` - The height of the rectangle.
/// * `color` - The 32-bit ARGB color value.
///
/// ## Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::rasterizer::rect::draw_rect;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// draw_rect(&mut fb, 10, 10, 20, 20, 0x00FF_FFFFFF);
/// ```
pub fn draw_rect(fb: &mut Framebuffer, x: i32, y: i32, width: u32, height: u32, color: u32) {
    if width == 0 || height == 0 {
        return;
    }

    // Safely calculate bounds preventing integer overflow on massive widths/heights
    let right = x
        .saturating_add(width.min(i32::MAX as u32) as i32)
        .saturating_sub(1);
    let bottom = y
        .saturating_add(height.min(i32::MAX as u32) as i32)
        .saturating_sub(1);

    // ⚡ Bolt: Early rejection for fully off-screen rects
    if right < 0 || bottom < 0 || x >= fb.width() as i32 || y >= fb.height() as i32 {
        return;
    }

    let is_on_screen = x >= 0 && y >= 0 && right < fb.width() as i32 && bottom < fb.height() as i32;

    if is_on_screen {
        // Top
        draw_horizontal_line_unchecked(fb, x, right, y, color);
        // Bottom
        draw_horizontal_line_unchecked(fb, x, right, bottom, color);
        // Left
        draw_vertical_line_unchecked(fb, x, y, bottom, color);
        // Right
        draw_vertical_line_unchecked(fb, right, y, bottom, color);
    } else {
        // Top
        draw_horizontal_line(fb, x, right, y, color);
        // Bottom
        draw_horizontal_line(fb, x, right, bottom, color);
        // Left
        draw_vertical_line(fb, x, y, bottom, color);
        // Right
        draw_vertical_line(fb, right, y, bottom, color);
    }
}

/// Fills a solid 2D rectangle.
///
/// Under the hood, this delegates to the framebuffer's internal fast block clearing routine.
/// It's designed to be the fastest way to draw solid UI backgrounds or wipe specific regions
/// of the screen, bypassing all edge-walking and rasterization logic entirely.
///
/// ## Parameters
///
/// * `fb` - The target framebuffer.
/// * `x` - The left X coordinate.
/// * `y` - The top Y coordinate.
/// * `width` - The width of the rectangle.
/// * `height` - The height of the rectangle.
/// * `color` - The 32-bit ARGB color value.
///
/// ## Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::rasterizer::rect::fill_rect;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// fill_rect(&mut fb, 10, 10, 20, 20, 0x00FF_FFFFFF);
/// ```
pub fn fill_rect(fb: &mut Framebuffer, x: i32, y: i32, width: u32, height: u32, color: u32) {
    fb.clear_rect(x, y, width, height, color);
}

/// Draws the outline of a 2D rounded rectangle.
///
/// This provides a more aesthetically pleasing alternative to standard rectangles,
/// perfect for modern UI elements like buttons or tooltips. It elegantly combines
/// straight edge lines with Bresenham's circle drawing algorithm for the corners,
/// ensuring a smooth curve without floating-point math overhead.
///
/// ## Parameters
///
/// * `fb` - The target framebuffer.
/// * `x` - The left X coordinate.
/// * `y` - The top Y coordinate.
/// * `width` - The overall width of the rectangle.
/// * `height` - The overall height of the rectangle.
/// * `radius` - The corner radius. Clamped to half the smallest dimension.
/// * `color` - The 32-bit ARGB color value.
///
/// ## Edge Cases
///
/// If `radius <= 0`, it degrades to a standard `draw_rect`.
///
/// ## Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::rasterizer::rect::draw_rounded_rect;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// draw_rounded_rect(&mut fb, 10, 10, 50, 50, 10, 0x00FF_FFFFFF);
/// ```
pub fn draw_rounded_rect(
    fb: &mut Framebuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    radius: i32,
    color: u32,
) {
    if width == 0 || height == 0 {
        return;
    }

    let w_i32 = width.min(i32::MAX as u32) as i32;
    let h_i32 = height.min(i32::MAX as u32) as i32;

    // ⚡ Bolt: Early rejection for fully off-screen rects
    let right = x.saturating_add(w_i32).saturating_sub(1);
    let bottom = y.saturating_add(h_i32).saturating_sub(1);
    if right < 0 || bottom < 0 || x >= fb.width() as i32 || y >= fb.height() as i32 {
        return;
    }

    let radius = radius.max(0).min(w_i32 / 2).min(h_i32 / 2).min(16384);

    if radius == 0 {
        draw_rect(fb, x, y, width, height, color);
        return;
    }

    let inner_w = w_i32.saturating_sub(2 * radius);
    let inner_h = h_i32.saturating_sub(2 * radius);

    let cx_left = x.saturating_add(radius);
    let cx_right = x
        .saturating_add(w_i32)
        .saturating_sub(1)
        .saturating_sub(radius);
    let cy_top = y.saturating_add(radius);
    let cy_bottom = y
        .saturating_add(h_i32)
        .saturating_sub(1)
        .saturating_sub(radius);

    let is_on_screen = x >= 0
        && y >= 0
        && x.saturating_add(w_i32) <= fb.width() as i32
        && y.saturating_add(h_i32) <= fb.height() as i32;

    // Draw straight edges
    if is_on_screen {
        if inner_w > 0 {
            draw_horizontal_line_unchecked(fb, cx_left, cx_right, y, color); // Top
            draw_horizontal_line_unchecked(fb, cx_left, cx_right, y + height as i32 - 1, color); // Bottom
        }
        if inner_h > 0 {
            draw_vertical_line_unchecked(fb, x, cy_top, cy_bottom, color); // Left
            draw_vertical_line_unchecked(fb, x + width as i32 - 1, cy_top, cy_bottom, color); // Right
        }
    } else {
        if inner_w > 0 {
            draw_horizontal_line(fb, cx_left, cx_right, y, color); // Top
            draw_horizontal_line(fb, cx_left, cx_right, y + height as i32 - 1, color); // Bottom
        }
        if inner_h > 0 {
            draw_vertical_line(fb, x, cy_top, cy_bottom, color); // Left
            draw_vertical_line(fb, x + width as i32 - 1, cy_top, cy_bottom, color); // Right
        }
    }

    let mut cx = 0;
    let mut cy = radius;
    // Convert to i64 to prevent circle algorithm overflow for massive radii
    let mut cx = 0i64;
    let mut cy = i64::from(radius);
    let mut d = 3i64 - 2i64 * i64::from(radius);

    if is_on_screen {
        let w = fb.width() as i32;
        let buf = fb.as_mut_slice();

        while cx <= cy {
            // Optimized on-screen rendering without bounds checking
            // Top Left
            unsafe {
                *buf.get_unchecked_mut(
                    ((i64::from(cy_top) - cy) * i64::from(w) + (i64::from(cx_left) - cx)) as usize,
                ) = color;
                *buf.get_unchecked_mut(
                    ((i64::from(cy_top) - cx) * i64::from(w) + (i64::from(cx_left) - cy)) as usize,
                ) = color;

                // Top Right
                *buf.get_unchecked_mut(
                    ((i64::from(cy_top) - cy) * i64::from(w) + (i64::from(cx_right) + cx)) as usize,
                ) = color;
                *buf.get_unchecked_mut(
                    ((i64::from(cy_top) - cx) * i64::from(w) + (i64::from(cx_right) + cy)) as usize,
                ) = color;

                // Bottom Left
                *buf.get_unchecked_mut(
                    ((i64::from(cy_bottom) + cy) * i64::from(w) + (i64::from(cx_left) - cx))
                        as usize,
                ) = color;
                *buf.get_unchecked_mut(
                    ((i64::from(cy_bottom) + cx) * i64::from(w) + (i64::from(cx_left) - cy))
                        as usize,
                ) = color;

                // Bottom Right
                *buf.get_unchecked_mut(
                    ((i64::from(cy_bottom) + cy) * i64::from(w) + (i64::from(cx_right) + cx))
                        as usize,
                ) = color;
                *buf.get_unchecked_mut(
                    ((i64::from(cy_bottom) + cx) * i64::from(w) + (i64::from(cx_right) + cy))
                        as usize,
                ) = color;
            }

            if d < 0 {
                d = d + 4 * cx + 6;
            } else {
                d = d + 4 * (cx - cy) + 10;
                cy -= 1;
            }
            cx += 1;
        }
    } else {
        while cx <= cy {
            fb.set_pixel(
                (i64::from(cx_left) - cx) as i32,
                (i64::from(cy_top) - cy) as i32,
                color,
            );
            fb.set_pixel(
                (i64::from(cx_left) - cy) as i32,
                (i64::from(cy_top) - cx) as i32,
                color,
            );
            fb.set_pixel(
                (i64::from(cx_right) + cx) as i32,
                (i64::from(cy_top) - cy) as i32,
                color,
            );
            fb.set_pixel(
                (i64::from(cx_right) + cy) as i32,
                (i64::from(cy_top) - cx) as i32,
                color,
            );
            fb.set_pixel(
                (i64::from(cx_left) - cx) as i32,
                (i64::from(cy_bottom) + cy) as i32,
                color,
            );
            fb.set_pixel(
                (i64::from(cx_left) - cy) as i32,
                (i64::from(cy_bottom) + cx) as i32,
                color,
            );
            fb.set_pixel(
                (i64::from(cx_right) + cx) as i32,
                (i64::from(cy_bottom) + cy) as i32,
                color,
            );
            fb.set_pixel(
                (i64::from(cx_right) + cy) as i32,
                (i64::from(cy_bottom) + cx) as i32,
                color,
            );

            if d < 0 {
                d = d + 4 * cx + 6;
            } else {
                d = d + 4 * (cx - cy) + 10;
                cy -= 1;
            }
            cx += 1;
        }
    }
}

/// Fills a solid 2D rounded rectangle.
///
/// While `draw_rounded_rect` is great for borders, this function fills the entire shape.
/// To maximize performance, it uses the fast block-clearing routine for the central region
/// and horizontal spans for the curved corners, minimizing memory writes and avoiding
/// expensive per-pixel boundary checks when the shape is fully on-screen.
///
/// ## Parameters
///
/// * `fb` - The target framebuffer.
/// * `x` - The left X coordinate.
/// * `y` - The top Y coordinate.
/// * `width` - The overall width of the rectangle.
/// * `height` - The overall height of the rectangle.
/// * `radius` - The corner radius. Clamped to half the smallest dimension.
/// * `color` - The 32-bit ARGB color value.
///
/// ## Edge Cases
///
/// If `radius <= 0`, it degrades to a standard `fill_rect`.
///
/// ## Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_render::rasterizer::rect::fill_rounded_rect;
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// fill_rounded_rect(&mut fb, 10, 10, 50, 50, 10, 0x00FF_FFFFFF);
/// ```
pub fn fill_rounded_rect(
    fb: &mut Framebuffer,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    radius: i32,
    color: u32,
) {
    if width == 0 || height == 0 {
        return;
    }

    let w_i32 = width.min(i32::MAX as u32) as i32;
    let h_i32 = height.min(i32::MAX as u32) as i32;

    // ⚡ Bolt: Early rejection for fully off-screen rects
    let right = x.saturating_add(w_i32).saturating_sub(1);
    let bottom = y.saturating_add(h_i32).saturating_sub(1);
    if right < 0 || bottom < 0 || x >= fb.width() as i32 || y >= fb.height() as i32 {
        return;
    }

    let radius = radius.max(0).min(w_i32 / 2).min(h_i32 / 2).min(16384);

    if radius == 0 {
        fill_rect(fb, x, y, width, height, color);
        return;
    }

    // Fill the central inner rectangle block
    fill_rect(
        fb,
        x,
        y.saturating_add(radius),
        width,
        height.saturating_sub(2 * (radius as u32)),
        color,
    );

    let cx_left = x.saturating_add(radius);
    let cx_right = x
        .saturating_add(w_i32)
        .saturating_sub(1)
        .saturating_sub(radius);
    let cy_top = y.saturating_add(radius);
    let cy_bottom = y
        .saturating_add(h_i32)
        .saturating_sub(1)
        .saturating_sub(radius);

    let is_on_screen = x >= 0
        && y >= 0
        && x.saturating_add(w_i32) <= fb.width() as i32
        && y.saturating_add(h_i32) <= fb.height() as i32;

    let mut cx = 0;
    let mut cy = radius;
    // Convert to i64 to prevent circle algorithm overflow for massive radii
    let mut cx = 0i64;
    let mut cy = i64::from(radius);
    let mut d = 3i64 - 2i64 * i64::from(radius);

    if is_on_screen {
        while cx <= cy {
            // Draw lines for corners, bypassing boundaries checks since we know it's on screen
            draw_horizontal_line_unchecked(
                fb,
                (i64::from(cx_left) - cx) as i32,
                (i64::from(cx_right) + cx) as i32,
                (i64::from(cy_top) - cy) as i32,
                color,
            );
            draw_horizontal_line_unchecked(
                fb,
                (i64::from(cx_left) - cx) as i32,
                (i64::from(cx_right) + cx) as i32,
                (i64::from(cy_bottom) + cy) as i32,
                color,
            );

            // To avoid overdraw on the middle portions if cx != cy
            if cx != cy {
                draw_horizontal_line_unchecked(
                    fb,
                    (i64::from(cx_left) - cy) as i32,
                    (i64::from(cx_right) + cy) as i32,
                    (i64::from(cy_top) - cx) as i32,
                    color,
                );
                draw_horizontal_line_unchecked(
                    fb,
                    (i64::from(cx_left) - cy) as i32,
                    (i64::from(cx_right) + cy) as i32,
                    (i64::from(cy_bottom) + cx) as i32,
                    color,
                );
            }

            if d < 0 {
                d = d + 4 * cx + 6;
            } else {
                d = d + 4 * (cx - cy) + 10;
                cy -= 1;
            }
            cx += 1;
        }
    } else {
        while cx <= cy {
            draw_horizontal_line(
                fb,
                (i64::from(cx_left) - cx) as i32,
                (i64::from(cx_right) + cx) as i32,
                (i64::from(cy_top) - cy) as i32,
                color,
            );
            draw_horizontal_line(
                fb,
                (i64::from(cx_left) - cx) as i32,
                (i64::from(cx_right) + cx) as i32,
                (i64::from(cy_bottom) + cy) as i32,
                color,
            );

            if cx != cy {
                draw_horizontal_line(
                    fb,
                    (i64::from(cx_left) - cy) as i32,
                    (i64::from(cx_right) + cy) as i32,
                    (i64::from(cy_top) - cx) as i32,
                    color,
                );
                draw_horizontal_line(
                    fb,
                    (i64::from(cx_left) - cy) as i32,
                    (i64::from(cx_right) + cy) as i32,
                    (i64::from(cy_bottom) + cx) as i32,
                    color,
                );
            }

            if d < 0 {
                d = d + 4 * cx + 6;
            } else {
                d = d + 4 * (cx - cy) + 10;
                cy -= 1;
            }
            cx += 1;
        }
    }
}

fn draw_line_2d_local(fb: &mut Framebuffer, p0: IVec2, p1: IVec2, color: u32) {
    let mut x0 = p0.x;
    let mut y0 = p0.y;
    let x1 = p1.x;
    let y1 = p1.y;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        fb.set_pixel(x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_rect() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        draw_rect(&mut fb, 10, 10, 20, 20, 0x00FF_FFFFFF);
    }

    #[test]
    fn test_fill_rect() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fill_rect(&mut fb, 10, 10, 20, 20, 0x00FF_FFFFFF);
    }

    #[test]
    fn test_draw_rounded_rect() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        draw_rounded_rect(&mut fb, 10, 10, 20, 20, 5, 0x00FF_FFFFFF);
    }

    #[test]
    fn test_fill_rounded_rect() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fill_rounded_rect(&mut fb, 10, 10, 20, 20, 5, 0x00FF_FFFFFF);
    }

    #[test]
    fn test_rounded_rect_oob() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        fill_rounded_rect(&mut fb, -10, -10, 50, 50, 10, 0x00FF_FFFFFF);
        draw_rounded_rect(&mut fb, 80, 80, 50, 50, 10, 0x00FF_FFFFFF);
    }

    #[test]
    fn test_draw_rect_oob() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        draw_rect(&mut fb, -10, -10, 50, 50, 0xFFFF_FFFF);
        draw_rect(&mut fb, 80, 80, 50, 50, 0xFFFF_FFFF);
    }
}
