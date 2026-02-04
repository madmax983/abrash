use crate::framebuffer::Framebuffer;

#[derive(Clone, Copy, PartialEq, Eq)]
struct OutCode(u8);

impl OutCode {
    const INSIDE: u8 = 0;
    const LEFT: u8 = 1;
    const RIGHT: u8 = 2;
    const BOTTOM: u8 = 4;
    const TOP: u8 = 8;

    fn compute(x: i32, y: i32, width: i32, height: i32) -> Self {
        let mut code = 0;
        if x < 0 {
            code |= Self::LEFT;
        } else if x >= width {
            code |= Self::RIGHT;
        }
        if y < 0 {
            code |= Self::BOTTOM;
        } else if y >= height {
            code |= Self::TOP;
        }
        OutCode(code)
    }

    fn is_inside(self) -> bool {
        self.0 == Self::INSIDE
    }

    fn shares_outside(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    fn contains(self, mask: u8) -> bool {
        (self.0 & mask) != 0
    }
}

/// Clip a line to the screen rectangle using Cohen-Sutherland algorithm.
/// Returns true if the line is at least partially visible.
fn clip_line(
    width: i32,
    height: i32,
    x0: &mut i32,
    y0: &mut i32,
    x1: &mut i32,
    y1: &mut i32,
) -> bool {
    let mut outcode0 = OutCode::compute(*x0, *y0, width, height);
    let mut outcode1 = OutCode::compute(*x1, *y1, width, height);
    let mut accept = false;

    loop {
        if outcode0.is_inside() && outcode1.is_inside() {
            // Both inside
            accept = true;
            break;
        } else if outcode0.shares_outside(outcode1) {
            // Both share an outside zone (trivial reject)
            break;
        } else {
            // Calculate intersection point
            let x: i32;
            let y: i32;

            // Pick at least one point outside
            let outcode_out = if !outcode0.is_inside() {
                outcode0
            } else {
                outcode1
            };

            // Using floating point for precision in intersection
            let x0_f = *x0 as f32;
            let y0_f = *y0 as f32;
            let x1_f = *x1 as f32;
            let y1_f = *y1 as f32;

            if outcode_out.contains(OutCode::TOP) {
                // Point is above clip window (y >= height)
                x = (x0_f + (x1_f - x0_f) * (height as f32 - 1.0 - y0_f) / (y1_f - y0_f)) as i32;
                y = height - 1;
            } else if outcode_out.contains(OutCode::BOTTOM) {
                // Point is below clip window (y < 0)
                x = (x0_f + (x1_f - x0_f) * (0.0 - y0_f) / (y1_f - y0_f)) as i32;
                y = 0;
            } else if outcode_out.contains(OutCode::RIGHT) {
                // Point is to the right of clip window (x >= width)
                y = (y0_f + (y1_f - y0_f) * (width as f32 - 1.0 - x0_f) / (x1_f - x0_f)) as i32;
                x = width - 1;
            } else {
                // LEFT
                // Point is to the left of clip window (x < 0)
                y = (y0_f + (y1_f - y0_f) * (0.0 - x0_f) / (x1_f - x0_f)) as i32;
                x = 0;
            }

            if outcode_out == outcode0 {
                *x0 = x;
                *y0 = y;
                outcode0 = OutCode::compute(*x0, *y0, width, height);
            } else {
                *x1 = x;
                *y1 = y;
                outcode1 = OutCode::compute(*x1, *y1, width, height);
            }
        }
    }
    accept
}

/// Draw a horizontal line (optimized - uses slice fill)
pub fn draw_hline(fb: &mut Framebuffer, x0: i32, x1: i32, y: i32, color: u32) {
    if y < 0 || y >= fb.height() as i32 {
        return;
    }

    let x_start = x0.min(x1).max(0).min(fb.width() as i32 - 1);
    let x_end = x0.max(x1).max(0).min(fb.width() as i32 - 1);

    let y = y as usize;
    let width = fb.width() as usize;
    let start = y * width + x_start as usize;
    let end = y * width + x_end as usize + 1;

    let pixels = fb.as_mut_slice();
    pixels[start..end].fill(color);
}

/// Draw a vertical line (optimized - stride access)
pub fn draw_vline(fb: &mut Framebuffer, x: i32, y0: i32, y1: i32, color: u32) {
    if x < 0 || x >= fb.width() as i32 {
        return;
    }

    let y_start = y0.min(y1).max(0).min(fb.height() as i32 - 1);
    let y_end = y0.max(y1).max(0).min(fb.height() as i32 - 1);

    let x = x as usize;
    let width = fb.width() as usize;
    let pixels = fb.as_mut_slice();

    for y in y_start..=y_end {
        let idx = y as usize * width + x;
        pixels[idx] = color;
    }
}

/// Draw a line using Bresenham's algorithm (internal, unchecked)
/// # Safety
/// Caller must ensure coordinates are within bounds.
unsafe fn draw_line_bresenham_unchecked(
    fb: &mut Framebuffer,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: u32,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        // SAFETY: Caller guarantees bounds.
        unsafe {
            fb.set_pixel_unchecked(x as usize, y as usize, color);
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;

        if e2 >= dy {
            if x == x1 {
                break;
            }
            err += dy;
            x += sx;
        }

        if e2 <= dx {
            if y == y1 {
                break;
            }
            err += dx;
            y += sy;
        }
    }
}

/// Draw a line using the best available method
pub fn draw_line(fb: &mut Framebuffer, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
    // Dispatch to optimized versions for axis-aligned lines
    if y0 == y1 {
        draw_hline(fb, x0, x1, y0, color);
        return;
    }
    if x0 == x1 {
        draw_vline(fb, x0, y0, y1, color);
        return;
    }

    // Fall back to Bresenham for diagonal lines
    let mut x0 = x0;
    let mut y0 = y0;
    let mut x1 = x1;
    let mut y1 = y1;

    if clip_line(
        fb.width() as i32,
        fb.height() as i32,
        &mut x0,
        &mut y0,
        &mut x1,
        &mut y1,
    ) {
        unsafe {
            draw_line_bresenham_unchecked(fb, x0, y0, x1, y1, color);
        }
    }
}
