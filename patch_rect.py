import re

with open('crates/abrash-render/src/rasterizer/rect.rs', 'r') as f:
    content = f.read()

# Add safety block to draw_horizontal_line
old_draw_h = """    let start_idx = y as usize * w + x_start as usize;
    let end_idx = y as usize * w + x_end as usize;

    // Explicitly avoids per-pixel bounds checks inside the slice
    fb.as_mut_slice()[start_idx..=end_idx].fill(color);"""
new_draw_h = """    let start_idx = y as usize * w + x_start as usize;
    let end_idx = y as usize * w + x_end as usize;

    // ⚡ Bolt: Explicitly avoids per-pixel bounds checks inside the slice
    // Safety: `y` is checked against `fb.height()`.
    // `x_start` and `x_end` are clamped between `0` and `fb.width() - 1`.
    // Therefore `start_idx` and `end_idx` are guaranteed to be within the slice bounds.
    unsafe {
        fb.as_mut_slice()
            .get_unchecked_mut(start_idx..=end_idx)
            .fill(color);
    }"""
content = content.replace(old_draw_h, new_draw_h)

# Add safety block to draw_horizontal_line_unchecked
old_draw_h_u = """    // Safety check fallback to standard fill if things went horribly wrong,
    // though the contract says it should be safe.
    if end_idx < fb.as_mut_slice().len() && start_idx <= end_idx {
        fb.as_mut_slice()[start_idx..=end_idx].fill(color);
    }"""
new_draw_h_u = """    // ⚡ Bolt: Bypass slice bounds checking entirely when geometric boundaries are pre-verified.
    // Safety: The caller must guarantee that `x0`, `x1`, and `y` are within bounds.
    unsafe {
        fb.as_mut_slice()
            .get_unchecked_mut(start_idx..=end_idx)
            .fill(color);
    }"""
content = content.replace(old_draw_h_u, new_draw_h_u)

old_v = """/// ⚡ Bolt: Fast vertical line fill that avoids Bresenham overhead and steps by framebuffer width.
#[inline(always)]
fn draw_vertical_line"""
new_v = """/// ⚡ Bolt: Fast vertical line fill that avoids Bresenham overhead and steps by framebuffer width.
///
/// ## Safety
///
/// The caller must guarantee that the starting index and all subsequent stepped indices
/// remain within the valid bounds of the framebuffer slice.
#[inline(always)]
fn draw_vertical_line"""
content = content.replace(old_v, new_v)

old_v_body = """    for _ in y_start..=y_end {
        buf[idx] = color;
        idx += w;
    }"""
new_v_body = """    for _ in y_start..=y_end {
        // Safety: `x` is checked against `fb.width()`.
        // `y_start` and `y_end` are clamped between `0` and `fb.height() - 1`.
        // Therefore `idx` is guaranteed to be within the slice bounds.
        unsafe {
            *buf.get_unchecked_mut(idx) = color;
        }
        idx += w;
    }"""
content = content.replace(old_v_body, new_v_body)


# also we need to mark unchecked function as unsafe
old_unchecked_decl = """fn draw_horizontal_line_unchecked(fb: &mut Framebuffer, x0: i32, x1: i32, y: i32, color: u32) {"""
new_unchecked_decl = """///
/// ## Safety
/// The caller must guarantee that the span is on screen.
unsafe fn draw_horizontal_line_unchecked(fb: &mut Framebuffer, x0: i32, x1: i32, y: i32, color: u32) {"""
content = content.replace(old_unchecked_decl, new_unchecked_decl)

old_unchecked_call_1 = """            // Draw lines for corners, bypassing boundaries checks since we know it's on screen
            draw_horizontal_line_unchecked(fb, cx_left - cx, cx_right + cx, cy_top - cy, color);
            draw_horizontal_line_unchecked(fb, cx_left - cx, cx_right + cx, cy_bottom + cy, color);

            // To avoid overdraw on the middle portions if cx != cy
            if cx != cy {
                draw_horizontal_line_unchecked(fb, cx_left - cy, cx_right + cy, cy_top - cx, color);
                draw_horizontal_line_unchecked(
                    fb,
                    cx_left - cy,
                    cx_right + cy,
                    cy_bottom + cx,
                    color,
                );
            }"""
new_unchecked_call_1 = """            // Draw lines for corners, bypassing boundaries checks since we know it's on screen
            unsafe {
                draw_horizontal_line_unchecked(fb, cx_left - cx, cx_right + cx, cy_top - cy, color);
                draw_horizontal_line_unchecked(fb, cx_left - cx, cx_right + cx, cy_bottom + cy, color);

                // To avoid overdraw on the middle portions if cx != cy
                if cx != cy {
                    draw_horizontal_line_unchecked(fb, cx_left - cy, cx_right + cy, cy_top - cx, color);
                    draw_horizontal_line_unchecked(
                        fb,
                        cx_left - cy,
                        cx_right + cy,
                        cy_bottom + cx,
                        color,
                    );
                }
            }"""
content = content.replace(old_unchecked_call_1, new_unchecked_call_1)

with open('crates/abrash-render/src/rasterizer/rect.rs', 'w') as f:
    f.write(content)
