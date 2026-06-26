//! Fast 2D bitmap blitting operations.
//!
//! The blitter provides three tiers of blit operations, each adding capabilities:
//!
//! 1. **Opaque blit** (`blit_opaque`) -- Straight memory copy, no alpha handling.
//!    Fastest path for fully opaque sprites and UI elements.
//!
//! 2. **Alpha-tested blit** (`blit_colorkey`) -- Skips pixels matching a color key.
//!    Used for sprites with transparent backgrounds (binary transparency).
//!
//! 3. **Alpha-blended blit** (`blit_alpha`) -- Per-pixel alpha blending using the
//!    source pixel's alpha channel. Supports smooth transparency and anti-aliased edges.
//!
//! All three tiers share the same clipping logic via `clip_blit`, which handles
//! negative destination coordinates, framebuffer bounds, and source texture bounds
//! before any pixels are touched.

use crate::framebuffer::Framebuffer;
use crate::texture::Texture;

/// A rectangular region within a source texture (atlas sub-rectangle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrcRect {
    /// X offset into the source texture in pixels.
    pub x: u32,
    /// Y offset into the source texture in pixels.
    pub y: u32,
    /// Width of the region in pixels.
    pub w: u32,
    /// Height of the region in pixels.
    pub h: u32,
}

/// The result of clipping a blit operation against framebuffer and texture bounds.
///
/// All coordinates are guaranteed to be within both the framebuffer and texture,
/// so the actual blit loop can run without any per-pixel bounds checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClippedBlit {
    /// X offset into the source texture to start reading.
    pub src_x: u32,
    /// Y offset into the source texture to start reading.
    pub src_y: u32,
    /// X offset into the destination framebuffer to start writing.
    pub dst_x: u32,
    /// Y offset into the destination framebuffer to start writing.
    pub dst_y: u32,
    /// Width of the clipped region in pixels.
    pub w: u32,
    /// Height of the clipped region in pixels.
    pub h: u32,
}

/// Clips a blit operation against both framebuffer and texture bounds.
///
/// Given a source rectangle within a texture and a signed destination position
/// in the framebuffer, computes the visible region that lies within both the
/// framebuffer `(fb_w, fb_h)` and the texture `(tex_w, tex_h)`.
///
/// Returns `None` if the blit is fully off-screen, has zero size, or the source
/// rectangle lies entirely outside the texture.
///
/// # Arguments
///
/// * `src` - Source rectangle within the texture.
/// * `dst_x` - Signed X destination in the framebuffer (negative = partially off-screen left).
/// * `dst_y` - Signed Y destination in the framebuffer (negative = partially off-screen top).
/// * `fb_w` - Framebuffer width in pixels.
/// * `fb_h` - Framebuffer height in pixels.
/// * `tex_w` - Source texture width in pixels.
/// * `tex_h` - Source texture height in pixels.
pub(crate) const fn clip_blit(
    src: &SrcRect,
    dst_x: i32,
    dst_y: i32,
    fb_w: u32,
    fb_h: u32,
    tex_w: u32,
    tex_h: u32,
) -> Option<ClippedBlit> {
    // Start with the full source rect.
    let mut sx = src.x;
    let mut sy = src.y;
    let mut w = src.w;
    let mut h = src.h;
    let mut dx = dst_x;
    let mut dy = dst_y;

    // Early-out: zero-size source.
    if w == 0 || h == 0 {
        return None;
    }

    // Clamp source rect to texture bounds.
    if sx >= tex_w || sy >= tex_h {
        return None;
    }
    if sx + w > tex_w {
        w = tex_w - sx;
    }
    if sy + h > tex_h {
        h = tex_h - sy;
    }

    // Left overflow: dst_x < 0 means we skip pixels on the left of the source.
    if dx < 0 {
        let skip = (-dx) as u32;
        if skip >= w {
            return None;
        }
        sx += skip;
        w -= skip;
        dx = 0;
    }

    // Top overflow: dst_y < 0 means we skip pixels on the top of the source.
    if dy < 0 {
        let skip = (-dy) as u32;
        if skip >= h {
            return None;
        }
        sy += skip;
        h -= skip;
        dy = 0;
    }

    // At this point dx and dy are non-negative.
    let dx = dx as u32;
    let dy = dy as u32;

    // Right overflow: clamp width to framebuffer right edge.
    if dx >= fb_w {
        return None;
    }
    if dx + w > fb_w {
        w = fb_w - dx;
    }

    // Bottom overflow: clamp height to framebuffer bottom edge.
    if dy >= fb_h {
        return None;
    }
    if dy + h > fb_h {
        h = fb_h - dy;
    }

    // Final zero-size check after all clipping.
    if w == 0 || h == 0 {
        return None;
    }

    Some(ClippedBlit {
        src_x: sx,
        src_y: sy,
        dst_x: dx,
        dst_y: dy,
        w,
        h,
    })
}

/// Blits a rectangular region of a texture onto the framebuffer with no alpha
/// handling -- a straight `memcpy` per row.
///
/// This is the fastest blit path: the source rectangle is clipped against both
/// the framebuffer and texture bounds via `clip_blit`, and then each scanline
/// is copied with `copy_from_slice` (a single `memcpy` under the hood).
///
/// Use this for fully opaque sprites, tilesets, and UI elements where every
/// source pixel overwrites the destination unconditionally.
///
/// # Arguments
///
/// * `fb` - Destination framebuffer.
/// * `tex` - Source texture (or atlas) to read from.
/// * `src` - Sub-rectangle within `tex` to copy.
/// * `dst_x` - Signed X position in the framebuffer (negative = partially off-screen left).
/// * `dst_y` - Signed Y position in the framebuffer (negative = partially off-screen top).
pub fn blit_opaque(fb: &mut Framebuffer, tex: &Texture, src: SrcRect, dst_x: i32, dst_y: i32) {
    let Some(clip) = clip_blit(
        &src,
        dst_x,
        dst_y,
        fb.width(),
        fb.height(),
        tex.width,
        tex.height,
    ) else {
        return;
    };

    let clipped_src = SrcRect {
        x: clip.src_x,
        y: clip.src_y,
        w: clip.w,
        h: clip.h,
    };

    // SAFETY: clip_blit guarantees all coordinates are within framebuffer and
    // texture bounds, satisfying the precondition of blit_opaque_unchecked.
    unsafe {
        blit_opaque_unchecked(fb, tex, clipped_src, clip.dst_x, clip.dst_y);
    }
}

/// Blits a rectangular region of a texture onto the framebuffer **without any
/// bounds checking**.
///
/// # Safety
///
/// The caller **must** guarantee that the entire source rectangle
/// `[src.x .. src.x + src.w, src.y .. src.y + src.h]` lies within
/// `tex.pixels`, and the entire destination rectangle
/// `[dst_x .. dst_x + src.w, dst_y .. dst_y + src.h]` lies within the
/// framebuffer. Violating this causes out-of-bounds memory access.
///
/// Prefer [`blit_opaque`] which clips automatically. Use this variant only
/// in hot inner loops where profiling proves the clipping check is a
/// measurable overhead.
pub unsafe fn blit_opaque_unchecked(
    fb: &mut Framebuffer,
    tex: &Texture,
    src: SrcRect,
    dst_x: u32,
    dst_y: u32,
) {
    let fb_w = fb.width() as usize;
    let tex_w = tex.width as usize;
    let w = src.w as usize;
    let fb_pixels = fb.as_mut_slice();

    for row in 0..src.h as usize {
        let src_offset = (src.y as usize + row) * tex_w + src.x as usize;
        let dst_offset = (dst_y as usize + row) * fb_w + dst_x as usize;
        fb_pixels[dst_offset..dst_offset + w]
            .copy_from_slice(&tex.pixels[src_offset..src_offset + w]);
    }
}

/// Blits a rectangular region of a texture onto the framebuffer, skipping any
/// pixels whose value matches the color key (binary transparency).
///
/// This is the standard alpha-tested blit path: the source rectangle is clipped
/// against both the framebuffer and texture bounds via `clip_blit`, and then
/// each pixel is compared against `key`. Matching pixels are skipped (leaving
/// the framebuffer contents intact), while non-matching pixels overwrite the
/// destination.
///
/// Use this for sprites with a designated transparent color (e.g. magenta
/// `0xFFFF00FF`).
///
/// # Arguments
///
/// * `fb` - Destination framebuffer.
/// * `tex` - Source texture (or atlas) to read from.
/// * `src` - Sub-rectangle within `tex` to copy.
/// * `dst_x` - Signed X position in the framebuffer (negative = partially off-screen left).
/// * `dst_y` - Signed Y position in the framebuffer (negative = partially off-screen top).
/// * `key` - The color key: any source pixel equal to this value is skipped.
pub fn blit_colorkey(
    fb: &mut Framebuffer,
    tex: &Texture,
    src: SrcRect,
    dst_x: i32,
    dst_y: i32,
    key: u32,
) {
    let Some(clip) = clip_blit(
        &src,
        dst_x,
        dst_y,
        fb.width(),
        fb.height(),
        tex.width,
        tex.height,
    ) else {
        return;
    };

    let clipped_src = SrcRect {
        x: clip.src_x,
        y: clip.src_y,
        w: clip.w,
        h: clip.h,
    };

    // SAFETY: clip_blit guarantees all coordinates are within framebuffer and
    // texture bounds, satisfying the precondition of blit_colorkey_unchecked.
    unsafe {
        blit_colorkey_unchecked(fb, tex, clipped_src, clip.dst_x, clip.dst_y, key);
    }
}

/// Blits a rectangular region of a texture onto the framebuffer with color-key
/// transparency, **without any bounds checking**.
///
/// Pixels matching `key` are skipped; all others overwrite the destination.
///
/// # Safety
///
/// The caller **must** guarantee that the entire source rectangle
/// `[src.x .. src.x + src.w, src.y .. src.y + src.h]` lies within
/// `tex.pixels`, and the entire destination rectangle
/// `[dst_x .. dst_x + src.w, dst_y .. dst_y + src.h]` lies within the
/// framebuffer. Violating this causes out-of-bounds memory access.
///
/// Prefer [`blit_colorkey`] which clips automatically. Use this variant only
/// in hot inner loops where profiling proves the clipping check is a
/// measurable overhead.
pub unsafe fn blit_colorkey_unchecked(
    fb: &mut Framebuffer,
    tex: &Texture,
    src: SrcRect,
    dst_x: u32,
    dst_y: u32,
    key: u32,
) {
    let fb_w = fb.width() as usize;
    let tex_w = tex.width as usize;
    let w = src.w as usize;
    let fb_pixels = fb.as_mut_slice();

    for row in 0..src.h as usize {
        let src_row_start = (src.y as usize + row) * tex_w + src.x as usize;
        let dst_row_start = (dst_y as usize + row) * fb_w + dst_x as usize;
        for col in 0..w {
            let src_px = tex.pixels[src_row_start + col];
            if src_px != key {
                fb_pixels[dst_row_start + col] = src_px;
            }
        }
    }
}

/// Alpha-blend src over dst (src-over compositing).
/// Uses SWAR to blend R+B and G channels in parallel.
/// Output alpha is always 0xFF (framebuffer is final display surface).
#[inline(always)]
const fn alpha_blend_pixel(src: u32, dst: u32) -> u32 {
    let alpha = (src >> 24) & 0xFF;
    let inv_alpha = 255 - alpha;

    let src_rb = src & 0x00FF_00FF;
    let src_g = (src >> 8) & 0x00FF_00FF;
    let dst_rb = dst & 0x00FF_00FF;
    let dst_g = (dst >> 8) & 0x00FF_00FF;

    let rb = ((src_rb * alpha + dst_rb * inv_alpha) >> 8) & 0x00FF_00FF;
    let g = ((src_g * alpha + dst_g * inv_alpha) >> 8) & 0x00FF_00FF;

    rb | (g << 8) | 0xFF00_0000
}

/// Blits a rectangular region of a texture onto the framebuffer with per-pixel
/// alpha blending (src-over compositing).
///
/// Each source pixel's alpha channel controls how it blends with the existing
/// framebuffer contents:
///
/// - `alpha = 0xFF` (fully opaque): source overwrites destination (fast path).
/// - `alpha = 0x00` (fully transparent): destination is unchanged (fast path).
/// - `0 < alpha < 0xFF` (semi-transparent): SWAR alpha blend via `alpha_blend_pixel`.
///
/// The source rectangle is clipped against both the framebuffer and texture
/// bounds via `clip_blit` before any pixels are touched.
///
/// Use this for sprites with smooth transparency, anti-aliased edges, or
/// translucent effects like particles and UI overlays.
///
/// # Arguments
///
/// * `fb` - Destination framebuffer.
/// * `tex` - Source texture (or atlas) to read from.
/// * `src` - Sub-rectangle within `tex` to copy.
/// * `dst_x` - Signed X position in the framebuffer (negative = partially off-screen left).
/// * `dst_y` - Signed Y position in the framebuffer (negative = partially off-screen top).
pub fn blit_alpha(fb: &mut Framebuffer, tex: &Texture, src: SrcRect, dst_x: i32, dst_y: i32) {
    let Some(clip) = clip_blit(
        &src,
        dst_x,
        dst_y,
        fb.width(),
        fb.height(),
        tex.width,
        tex.height,
    ) else {
        return;
    };

    let clipped_src = SrcRect {
        x: clip.src_x,
        y: clip.src_y,
        w: clip.w,
        h: clip.h,
    };

    // SAFETY: clip_blit guarantees all coordinates are within framebuffer and
    // texture bounds, satisfying the precondition of blit_alpha_unchecked.
    unsafe {
        blit_alpha_unchecked(fb, tex, clipped_src, clip.dst_x, clip.dst_y);
    }
}

/// Blits a rectangular region of a texture onto the framebuffer with per-pixel
/// alpha blending, **without any bounds checking**.
///
/// Uses a three-way branch per pixel:
/// - Fully opaque (`alpha == 0xFF`): direct copy (no blend math).
/// - Fully transparent (`alpha == 0x00`): skip entirely.
/// - Semi-transparent: SWAR alpha blend via `alpha_blend_pixel`.
///
/// # Safety
///
/// The caller **must** guarantee that the entire source rectangle
/// `[src.x .. src.x + src.w, src.y .. src.y + src.h]` lies within
/// `tex.pixels`, and the entire destination rectangle
/// `[dst_x .. dst_x + src.w, dst_y .. dst_y + src.h]` lies within the
/// framebuffer. Violating this causes out-of-bounds memory access.
///
/// Prefer [`blit_alpha`] which clips automatically. Use this variant only
/// in hot inner loops where profiling proves the clipping check is a
/// measurable overhead.
pub unsafe fn blit_alpha_unchecked(
    fb: &mut Framebuffer,
    tex: &Texture,
    src: SrcRect,
    dst_x: u32,
    dst_y: u32,
) {
    let fb_w = fb.width() as usize;
    let tex_w = tex.width as usize;
    let w = src.w as usize;
    let fb_pixels = fb.as_mut_slice();

    for row in 0..src.h as usize {
        let src_row_start = (src.y as usize + row) * tex_w + src.x as usize;
        let dst_row_start = (dst_y as usize + row) * fb_w + dst_x as usize;
        for col in 0..w {
            let src_px = tex.pixels[src_row_start + col];
            let alpha = src_px >> 24;
            if alpha == 0xFF {
                fb_pixels[dst_row_start + col] = src_px; // fully opaque
            } else if alpha > 0 {
                fb_pixels[dst_row_start + col] =
                    alpha_blend_pixel(src_px, fb_pixels[dst_row_start + col]);
            }
            // alpha == 0 → skip
        }
    }
}

/// Fills a rectangle in the framebuffer with a solid color.
///
/// This is a thin wrapper around [`Framebuffer::clear_rect`] for API
/// consistency with the `blit_*` family. All clipping (negative coordinates,
/// framebuffer bounds) is handled by `clear_rect`.
///
/// # Arguments
///
/// * `fb` - Destination framebuffer.
/// * `x` - Signed X position (negative = partially off-screen left).
/// * `y` - Signed Y position (negative = partially off-screen top).
/// * `w` - Width of the rectangle in pixels.
/// * `h` - Height of the rectangle in pixels.
/// * `color` - Fill color in ARGB format.
pub fn fill_rect(fb: &mut Framebuffer, x: i32, y: i32, w: u32, h: u32, color: u32) {
    fb.clear_rect(x, y, w, h, color);
}

/// Fills a rectangle in the framebuffer with an alpha-blended color.
///
/// Uses the source color's alpha channel (bits 31..24) to blend with existing
/// framebuffer contents via `alpha_blend_pixel` (src-over compositing).
///
/// Fast paths:
/// - `alpha == 0`: no-op (fully transparent).
/// - `alpha == 0xFF`: delegates to [`Framebuffer::clear_rect`] (fully opaque).
///
/// The rectangle is clipped against the framebuffer bounds before any pixels
/// are touched. Negative `x` / `y` values are handled correctly.
///
/// # Arguments
///
/// * `fb` - Destination framebuffer.
/// * `x` - Signed X position (negative = partially off-screen left).
/// * `y` - Signed Y position (negative = partially off-screen top).
/// * `w` - Width of the rectangle in pixels.
/// * `h` - Height of the rectangle in pixels.
/// * `color` - Fill color in ARGB format (alpha in bits 31..24).
pub fn fill_rect_alpha(fb: &mut Framebuffer, x: i32, y: i32, w: u32, h: u32, color: u32) {
    if w == 0 || h == 0 {
        return;
    }
    let alpha = color >> 24;
    if alpha == 0 {
        return;
    }
    if alpha == 0xFF {
        fb.clear_rect(x, y, w, h, color);
        return;
    }

    let fb_w = fb.width() as i32;
    let fb_h = fb.height() as i32;

    let x0 = x.max(0) as u32;
    let y0 = y.max(0) as u32;
    let x1 = ((i64::from(x) + i64::from(w)).min(i64::from(fb_w)) as i32).max(0) as u32;
    let y1 = ((i64::from(y) + i64::from(h)).min(i64::from(fb_h)) as i32).max(0) as u32;

    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let stride = fb.width() as usize;
    let fb_pixels = fb.as_mut_slice();

    // Hoist loop-invariant source terms: color, alpha, and pre-multiplied
    // src channels are constant across every pixel in the fill.
    let src_rb = color & 0x00FF_00FF;
    let src_g = (color >> 8) & 0x00FF_00FF;
    let src_rb_a = src_rb * alpha;
    let src_g_a = src_g * alpha;
    let inv_alpha = 255 - alpha;

    for row in fb_pixels
        .chunks_exact_mut(stride)
        .skip(y0 as usize)
        .take((y1 - y0) as usize)
    {
        // SAFETY: x0 and x1 are clamped to fb.width() above (via max/min logic),
        // which matches stride.
        let row_slice = unsafe { row.get_unchecked_mut(x0 as usize..x1 as usize) };
        for dst in row_slice.iter_mut() {
            let d = *dst;
            let dst_rb = d & 0x00FF_00FF;
            let dst_g = (d >> 8) & 0x00FF_00FF;

            let rb = ((src_rb_a + dst_rb * inv_alpha) >> 8) & 0x00FF_00FF;
            let g = ((src_g_a + dst_g * inv_alpha) >> 8) & 0x00FF_00FF;

            *dst = rb | (g << 8) | 0xFF00_0000;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_blit_no_clipping_needed() {
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };
        let result = clip_blit(&src, 10, 10, 800, 600, 64, 64).unwrap();
        assert_eq!(
            result,
            ClippedBlit {
                src_x: 0,
                src_y: 0,
                dst_x: 10,
                dst_y: 10,
                w: 16,
                h: 16,
            }
        );
    }

    #[test]
    fn clip_blit_left_overflow() {
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };
        let result = clip_blit(&src, -4, 10, 800, 600, 64, 64).unwrap();
        assert_eq!(result.src_x, 4);
        assert_eq!(result.dst_x, 0);
        assert_eq!(result.w, 12);
    }

    #[test]
    fn clip_blit_top_overflow() {
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };
        let result = clip_blit(&src, 10, -6, 800, 600, 64, 64).unwrap();
        assert_eq!(result.src_y, 6);
        assert_eq!(result.dst_y, 0);
        assert_eq!(result.h, 10);
    }

    #[test]
    fn clip_blit_right_overflow() {
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };
        let result = clip_blit(&src, 790, 10, 800, 600, 64, 64).unwrap();
        assert_eq!(result.w, 10);
        assert_eq!(result.dst_x, 790);
    }

    #[test]
    fn clip_blit_bottom_overflow() {
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };
        let result = clip_blit(&src, 10, 592, 800, 600, 64, 64).unwrap();
        assert_eq!(result.h, 8);
        assert_eq!(result.dst_y, 592);
    }

    #[test]
    fn clip_blit_fully_offscreen_left() {
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };
        assert!(clip_blit(&src, -20, 10, 800, 600, 64, 64).is_none());
    }

    #[test]
    fn clip_blit_fully_offscreen_right() {
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };
        assert!(clip_blit(&src, 800, 10, 800, 600, 64, 64).is_none());
    }

    #[test]
    fn clip_blit_fully_offscreen_top() {
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };
        assert!(clip_blit(&src, 10, -20, 800, 600, 64, 64).is_none());
    }

    #[test]
    fn clip_blit_fully_offscreen_bottom() {
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };
        assert!(clip_blit(&src, 10, 600, 800, 600, 64, 64).is_none());
    }

    #[test]
    fn clip_blit_zero_size_src() {
        let zero_w = SrcRect {
            x: 0,
            y: 0,
            w: 0,
            h: 16,
        };
        assert!(clip_blit(&zero_w, 10, 10, 800, 600, 64, 64).is_none());

        let zero_h = SrcRect {
            x: 0,
            y: 0,
            w: 16,
            h: 0,
        };
        assert!(clip_blit(&zero_h, 10, 10, 800, 600, 64, 64).is_none());
    }

    #[test]
    fn clip_blit_src_exceeds_texture() {
        let src = SrcRect {
            x: 60,
            y: 60,
            w: 16,
            h: 16,
        };
        let result = clip_blit(&src, 10, 10, 800, 600, 64, 64).unwrap();
        assert_eq!(result.src_x, 60);
        assert_eq!(result.src_y, 60);
        assert_eq!(result.w, 4);
        assert_eq!(result.h, 4);
    }

    #[test]
    fn clip_blit_src_fully_outside_texture() {
        let src = SrcRect {
            x: 64,
            y: 0,
            w: 16,
            h: 16,
        };
        assert!(clip_blit(&src, 10, 10, 800, 600, 64, 64).is_none());
    }

    #[test]
    fn clip_blit_corner_diagonal() {
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 32,
            h: 32,
        };
        let result = clip_blit(&src, -8, -8, 800, 600, 64, 64).unwrap();
        assert_eq!(
            result,
            ClippedBlit {
                src_x: 8,
                src_y: 8,
                dst_x: 0,
                dst_y: 0,
                w: 24,
                h: 24,
            }
        );
    }

    #[test]
    fn clip_blit_exact_fit() {
        let src = SrcRect {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
        };
        let result = clip_blit(&src, 0, 0, 100, 100, 100, 100).unwrap();
        assert_eq!(
            result,
            ClippedBlit {
                src_x: 0,
                src_y: 0,
                dst_x: 0,
                dst_y: 0,
                w: 100,
                h: 100,
            }
        );
    }

    #[test]
    fn clip_blit_1x1_sprite() {
        let src = SrcRect {
            x: 32,
            y: 32,
            w: 1,
            h: 1,
        };
        let result = clip_blit(&src, 400, 300, 800, 600, 64, 64).unwrap();
        assert_eq!(
            result,
            ClippedBlit {
                src_x: 32,
                src_y: 32,
                dst_x: 400,
                dst_y: 300,
                w: 1,
                h: 1,
            }
        );
    }

    #[test]
    fn clip_blit_atlas_subregion() {
        let src = SrcRect {
            x: 32,
            y: 16,
            w: 16,
            h: 16,
        };
        let result = clip_blit(&src, 795, 10, 800, 600, 64, 64).unwrap();
        assert_eq!(result.src_x, 32);
        assert_eq!(result.src_y, 16);
        assert_eq!(result.dst_x, 795);
        assert_eq!(result.w, 5);
        assert_eq!(result.h, 16);
    }

    // ── blit_opaque / blit_opaque_unchecked tests ──────────────────────

    use crate::framebuffer::Framebuffer;
    use crate::texture::Texture;

    fn make_checkerboard_texture(w: u32, h: u32) -> Texture {
        let mut tex = Texture::new(w, h).unwrap();
        for y in 0..h {
            for x in 0..w {
                let color = if (x + y) % 2 == 0 {
                    0xFFFF0000
                } else {
                    0xFF00FF00
                };
                tex.set_pixel(x, y, color);
            }
        }
        tex
    }

    #[test]
    fn blit_opaque_basic() {
        let tex = make_checkerboard_texture(4, 4);
        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(0xFF000000);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };
        blit_opaque(&mut fb, &tex, src, 2, 3);

        let fb_pixels = fb.as_slice();
        let fb_w = 16usize;

        // Verify all 16 blitted pixels match the checkerboard.
        for y in 0..4u32 {
            for x in 0..4u32 {
                let expected = if (x + y) % 2 == 0 {
                    0xFFFF0000
                } else {
                    0xFF00FF00
                };
                let idx = (3 + y as usize) * fb_w + (2 + x as usize);
                assert_eq!(
                    fb_pixels[idx],
                    expected,
                    "Mismatch at dst ({}, {})",
                    2 + x,
                    3 + y
                );
            }
        }

        // Verify surrounding pixels are untouched (black).
        assert_eq!(fb_pixels[0], 0xFF000000, "top-left corner should be black");
        assert_eq!(
            fb_pixels[fb_w - 1],
            0xFF000000,
            "top-right corner should be black"
        );
        assert_eq!(
            fb_pixels[(fb_w * 16) - 1],
            0xFF000000,
            "bottom-right should be black"
        );
    }

    #[test]
    fn blit_opaque_clipped_left() {
        let tex = make_checkerboard_texture(8, 8);
        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(0xFF000000);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        };
        // dst_x = -3 means columns 0..5 of fb get src columns 3..8.
        blit_opaque(&mut fb, &tex, src, -3, 0);

        let fb_pixels = fb.as_slice();
        let fb_w = 16usize;

        for row in 0..8u32 {
            for col in 0..5u32 {
                let src_x = col + 3; // skipped 3 columns
                let expected = if (src_x + row) % 2 == 0 {
                    0xFFFF0000
                } else {
                    0xFF00FF00
                };
                let idx = row as usize * fb_w + col as usize;
                assert_eq!(
                    fb_pixels[idx], expected,
                    "Mismatch at fb ({col}, {row}), src_x={src_x}"
                );
            }
        }

        // Column 5 onward should be black.
        assert_eq!(fb_pixels[5], 0xFF000000, "column 5 row 0 should be black");
    }

    #[test]
    fn blit_opaque_fully_offscreen() {
        let tex = make_checkerboard_texture(8, 8);
        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(0xFF000000);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        };
        blit_opaque(&mut fb, &tex, src, -10, 0);

        // Every pixel in the framebuffer should be untouched.
        for (i, &pixel) in fb.as_slice().iter().enumerate() {
            assert_eq!(pixel, 0xFF000000, "Pixel {i} should be black");
        }
    }

    #[test]
    fn blit_opaque_atlas_subregion() {
        let tex = make_checkerboard_texture(16, 16);
        let mut fb = Framebuffer::new(32, 32).unwrap();
        fb.clear(0xFF000000);

        // Blit the 4x4 sub-region starting at (4,4) to dst (10,10).
        let src = SrcRect {
            x: 4,
            y: 4,
            w: 4,
            h: 4,
        };
        blit_opaque(&mut fb, &tex, src, 10, 10);

        let fb_pixels = fb.as_slice();
        let fb_w = 32usize;

        for y in 0..4u32 {
            for x in 0..4u32 {
                let tex_x = x + 4;
                let tex_y = y + 4;
                let expected = if (tex_x + tex_y) % 2 == 0 {
                    0xFFFF0000
                } else {
                    0xFF00FF00
                };
                let idx = (10 + y as usize) * fb_w + (10 + x as usize);
                assert_eq!(
                    fb_pixels[idx],
                    expected,
                    "Mismatch at dst ({}, {})",
                    10 + x,
                    10 + y
                );
            }
        }

        // Spot-check surrounding area is still black.
        assert_eq!(fb_pixels[0], 0xFF000000);
        assert_eq!(fb_pixels[9 * fb_w + 9], 0xFF000000);
        assert_eq!(fb_pixels[14 * fb_w + 14], 0xFF000000);
    }

    #[test]
    fn blit_opaque_unchecked_basic() {
        let tex = make_checkerboard_texture(4, 4);
        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(0xFF000000);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };

        // SAFETY: dst (2,3) + size (4,4) = (6,7), well within 16x16 fb,
        // and src (0,0)+(4,4) is within 4x4 texture.
        unsafe {
            blit_opaque_unchecked(&mut fb, &tex, src, 2, 3);
        }

        let fb_pixels = fb.as_slice();
        let fb_w = 16usize;

        for y in 0..4u32 {
            for x in 0..4u32 {
                let expected = if (x + y) % 2 == 0 {
                    0xFFFF0000
                } else {
                    0xFF00FF00
                };
                let idx = (3 + y as usize) * fb_w + (2 + x as usize);
                assert_eq!(
                    fb_pixels[idx],
                    expected,
                    "Mismatch at dst ({}, {})",
                    2 + x,
                    3 + y
                );
            }
        }

        // Surrounding untouched.
        assert_eq!(fb_pixels[0], 0xFF000000);
        assert_eq!(
            fb_pixels[(fb_w * 16) - 1],
            0xFF000000,
            "bottom-right should be black"
        );
    }

    // ── blit_colorkey / blit_colorkey_unchecked tests ────────────────────

    #[test]
    fn blit_colorkey_skips_key_color() {
        const RED: u32 = 0xFFFF0000;
        const MAGENTA: u32 = 0xFFFF00FF; // color key
        const BLUE: u32 = 0xFF0000FF;

        // 4x4 texture: top-left 2x2 = red, rest = magenta (transparent).
        let mut tex = Texture::new(4, 4).unwrap();
        for y in 0..4u32 {
            for x in 0..4u32 {
                let color = if x < 2 && y < 2 { RED } else { MAGENTA };
                tex.set_pixel(x, y, color);
            }
        }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(BLUE);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };
        blit_colorkey(&mut fb, &tex, src, 5, 5, MAGENTA);

        let fb_pixels = fb.as_slice();
        let fb_w = 16usize;

        // Top-left 2x2 of the blit region should be red.
        for y in 0..2u32 {
            for x in 0..2u32 {
                let idx = (5 + y as usize) * fb_w + (5 + x as usize);
                assert_eq!(
                    fb_pixels[idx],
                    RED,
                    "pixel ({}, {}) should be red",
                    5 + x,
                    5 + y
                );
            }
        }

        // Remaining pixels in the blit region should still be blue (magenta was skipped).
        for y in 0..4u32 {
            for x in 0..4u32 {
                if x < 2 && y < 2 {
                    continue; // already checked
                }
                let idx = (5 + y as usize) * fb_w + (5 + x as usize);
                assert_eq!(
                    fb_pixels[idx],
                    BLUE,
                    "pixel ({}, {}) should be blue (key skipped)",
                    5 + x,
                    5 + y
                );
            }
        }

        // Pixels outside the blit region should be blue.
        assert_eq!(fb_pixels[0], BLUE, "top-left corner should be blue");
    }

    #[test]
    fn blit_colorkey_all_transparent() {
        const KEY: u32 = 0xFFFF00FF;
        const BLUE: u32 = 0xFF0000FF;

        // 4x4 texture: all pixels match the key.
        let mut tex = Texture::new(4, 4).unwrap();
        for y in 0..4u32 {
            for x in 0..4u32 {
                tex.set_pixel(x, y, KEY);
            }
        }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(BLUE);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };
        blit_colorkey(&mut fb, &tex, src, 5, 5, KEY);

        // Every pixel in the framebuffer should still be blue.
        for (i, &pixel) in fb.as_slice().iter().enumerate() {
            assert_eq!(pixel, BLUE, "pixel {i} should be blue (all transparent)");
        }
    }

    #[test]
    fn blit_colorkey_clipped() {
        const RED: u32 = 0xFFFF0000;
        const BLUE: u32 = 0xFF0000FF;

        // 8x8 all-red texture (no pixel matches the key, so all are opaque).
        let mut tex = Texture::new(8, 8).unwrap();
        for y in 0..8u32 {
            for x in 0..8u32 {
                tex.set_pixel(x, y, RED);
            }
        }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(BLUE);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        };
        // dst_x = -3 means columns 0..5 of fb get src columns 3..8.
        blit_colorkey(&mut fb, &tex, src, -3, 0, 0xFFFF00FF);

        let fb_pixels = fb.as_slice();
        let fb_w = 16usize;

        // Columns 0..5 in rows 0..8 should be red.
        for row in 0..8u32 {
            for col in 0..5u32 {
                let idx = row as usize * fb_w + col as usize;
                assert_eq!(fb_pixels[idx], RED, "pixel ({col}, {row}) should be red");
            }
        }

        // Column 5 onward should still be blue.
        for row in 0..8u32 {
            let idx = row as usize * fb_w + 5;
            assert_eq!(fb_pixels[idx], BLUE, "pixel (5, {row}) should be blue");
        }
    }

    #[test]
    fn blit_colorkey_unchecked_basic() {
        const RED: u32 = 0xFFFF0000;
        const MAGENTA: u32 = 0xFFFF00FF; // color key
        const BLUE: u32 = 0xFF0000FF;

        // 4x4 texture: first column = red, rest = magenta (transparent).
        let mut tex = Texture::new(4, 4).unwrap();
        for y in 0..4u32 {
            for x in 0..4u32 {
                let color = if x == 0 { RED } else { MAGENTA };
                tex.set_pixel(x, y, color);
            }
        }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(BLUE);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };

        // SAFETY: dst (2,3) + size (4,4) = (6,7), well within 16x16 fb,
        // and src (0,0)+(4,4) is within 4x4 texture.
        unsafe {
            blit_colorkey_unchecked(&mut fb, &tex, src, 2, 3, MAGENTA);
        }

        let fb_pixels = fb.as_slice();
        let fb_w = 16usize;

        // First column of the blit region should be red.
        for y in 0..4u32 {
            let idx = (3 + y as usize) * fb_w + 2;
            assert_eq!(fb_pixels[idx], RED, "pixel (2, {}) should be red", 3 + y);
        }

        // Remaining 3 columns of the blit region should be blue (magenta skipped).
        for y in 0..4u32 {
            for x in 1..4u32 {
                let idx = (3 + y as usize) * fb_w + (2 + x as usize);
                assert_eq!(
                    fb_pixels[idx],
                    BLUE,
                    "pixel ({}, {}) should be blue",
                    2 + x,
                    3 + y
                );
            }
        }

        // Surrounding pixels untouched.
        assert_eq!(fb_pixels[0], BLUE, "top-left corner should be blue");
        assert_eq!(
            fb_pixels[(fb_w * 16) - 1],
            BLUE,
            "bottom-right should be blue"
        );
    }

    // ── blit_alpha / blit_alpha_unchecked tests ─────────────────────────

    /// Extract a single color channel from an ARGB pixel.
    fn channel(color: u32, shift: u32) -> u32 {
        (color >> shift) & 0xFF
    }

    #[test]
    fn blit_alpha_fully_opaque() {
        const RED: u32 = 0xFFFF0000; // alpha = 0xFF
        const BLUE: u32 = 0xFF0000FF;

        let mut tex = Texture::new(4, 4).unwrap();
        for y in 0..4u32 {
            for x in 0..4u32 {
                tex.set_pixel(x, y, RED);
            }
        }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(BLUE);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };
        blit_alpha(&mut fb, &tex, src, 5, 5);

        let fb_pixels = fb.as_slice();
        let fb_w = 16usize;

        // All blitted pixels should be red (fully opaque overwrites).
        for y in 0..4u32 {
            for x in 0..4u32 {
                let idx = (5 + y as usize) * fb_w + (5 + x as usize);
                assert_eq!(
                    fb_pixels[idx],
                    RED,
                    "pixel ({}, {}) should be red",
                    5 + x,
                    5 + y
                );
            }
        }

        // Surrounding pixels should still be blue.
        assert_eq!(fb_pixels[0], BLUE, "top-left corner should be blue");
    }

    #[test]
    fn blit_alpha_fully_transparent() {
        const TRANSPARENT_RED: u32 = 0x00FF0000; // alpha = 0x00
        const BLUE: u32 = 0xFF0000FF;

        let mut tex = Texture::new(4, 4).unwrap();
        for y in 0..4u32 {
            for x in 0..4u32 {
                tex.set_pixel(x, y, TRANSPARENT_RED);
            }
        }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(BLUE);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 4,
            h: 4,
        };
        blit_alpha(&mut fb, &tex, src, 5, 5);

        // Every pixel in the framebuffer should still be blue.
        for (i, &pixel) in fb.as_slice().iter().enumerate() {
            assert_eq!(pixel, BLUE, "pixel {i} should be blue (all transparent)");
        }
    }

    #[test]
    fn blit_alpha_50_percent() {
        // 50% alpha red over opaque blue.
        const HALF_RED: u32 = 0x80FF0000; // alpha = 0x80 (128)
        const BLUE: u32 = 0xFF0000FF;

        let mut tex = Texture::new(1, 1).unwrap();
        tex.set_pixel(0, 0, HALF_RED);

        let mut fb = Framebuffer::new(4, 4).unwrap();
        fb.clear(BLUE);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 1,
            h: 1,
        };
        blit_alpha(&mut fb, &tex, src, 0, 0);

        let result = fb.as_slice()[0];

        // R channel: src=255, dst=0 -> blended ~128
        let r = channel(result, 16);
        assert!(r.abs_diff(128) <= 2, "R channel should be ~128, got {r}");

        // G channel: src=0, dst=0 -> blended 0
        let g = channel(result, 8);
        assert_eq!(g, 0, "G channel should be 0, got {g}");

        // B channel: src=0, dst=255 -> blended ~127
        let b = channel(result, 0);
        assert!(b.abs_diff(127) <= 2, "B channel should be ~127, got {b}");

        // A channel: always 0xFF for framebuffer output.
        let a = channel(result, 24);
        assert_eq!(a, 0xFF, "A channel should be 0xFF, got {a}");
    }

    #[test]
    fn blit_alpha_clipped() {
        const RED: u32 = 0xFFFF0000; // fully opaque red
        const BLUE: u32 = 0xFF0000FF;

        let mut tex = Texture::new(8, 8).unwrap();
        for y in 0..8u32 {
            for x in 0..8u32 {
                tex.set_pixel(x, y, RED);
            }
        }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(BLUE);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        };
        // dst (-3, -3): clips to 5x5 region at fb (0,0).
        blit_alpha(&mut fb, &tex, src, -3, -3);

        let fb_pixels = fb.as_slice();
        let fb_w = 16usize;

        // (0,0) should be red (within clipped region).
        assert_eq!(fb_pixels[0], RED, "(0,0) should be red");

        // (4,4) should be red (last pixel of clipped region).
        assert_eq!(fb_pixels[4 * fb_w + 4], RED, "(4,4) should be red");

        // (5,0) should be blue (outside clipped region).
        assert_eq!(fb_pixels[5], BLUE, "(5,0) should be blue");
    }

    #[test]
    fn blit_alpha_unchecked_basic() {
        const RED: u32 = 0xFFFF0000;
        const BLUE: u32 = 0xFF0000FF;

        let mut tex = Texture::new(2, 2).unwrap();
        for y in 0..2u32 {
            for x in 0..2u32 {
                tex.set_pixel(x, y, RED);
            }
        }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(BLUE);

        let src = SrcRect {
            x: 0,
            y: 0,
            w: 2,
            h: 2,
        };

        // SAFETY: dst (3,3) + size (2,2) = (5,5), well within 16x16 fb,
        // and src (0,0)+(2,2) is within 2x2 texture.
        unsafe {
            blit_alpha_unchecked(&mut fb, &tex, src, 3, 3);
        }

        let fb_pixels = fb.as_slice();
        let fb_w = 16usize;

        // All 4 blitted pixels should be red.
        for y in 0..2u32 {
            for x in 0..2u32 {
                let idx = (3 + y as usize) * fb_w + (3 + x as usize);
                assert_eq!(
                    fb_pixels[idx],
                    RED,
                    "pixel ({}, {}) should be red",
                    3 + x,
                    3 + y
                );
            }
        }

        // Surrounding pixels untouched.
        assert_eq!(fb_pixels[0], BLUE, "top-left corner should be blue");
        assert_eq!(
            fb_pixels[(fb_w * 16) - 1],
            BLUE,
            "bottom-right should be blue"
        );
    }

    // ── fill_rect / fill_rect_alpha tests ──────────────────────────────

    #[test]
    fn fill_rect_basic() {
        const RED: u32 = 0xFFFF0000;
        const DEFAULT: u32 = 0xFF000000;

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(DEFAULT);

        fill_rect(&mut fb, 2, 3, 4, 5, RED);

        let fb_pixels = fb.as_slice();
        let fb_w = 16usize;

        // Pixels inside the rect should be red.
        for y in 3..8u32 {
            for x in 2..6u32 {
                let idx = y as usize * fb_w + x as usize;
                assert_eq!(fb_pixels[idx], RED, "pixel ({x}, {y}) should be red");
            }
        }

        // Spot-check pixels outside the rect should be default.
        assert_eq!(fb_pixels[0], DEFAULT, "(0,0) should be default");
        assert_eq!(fb_pixels[1 * fb_w + 1], DEFAULT, "(1,1) should be default");
        assert_eq!(
            fb_pixels[8 * fb_w + 6],
            DEFAULT,
            "(6,8) should be default (one row below rect)"
        );
        assert_eq!(
            fb_pixels[3 * fb_w + 6],
            DEFAULT,
            "(6,3) should be default (one col right of rect)"
        );
    }

    #[test]
    fn fill_rect_clipped_negative() {
        const RED: u32 = 0xFFFF0000;
        const DEFAULT: u32 = 0xFF000000;

        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(DEFAULT);

        // Rect at (-2,-2) size 5x5 → visible region is (0..3, 0..3).
        fill_rect(&mut fb, -2, -2, 5, 5, RED);

        let fb_pixels = fb.as_slice();
        let fb_w = 10usize;

        // Pixels in (0..3, 0..3) should be red.
        for y in 0..3u32 {
            for x in 0..3u32 {
                let idx = y as usize * fb_w + x as usize;
                assert_eq!(fb_pixels[idx], RED, "pixel ({x}, {y}) should be red");
            }
        }

        // Pixels outside the clipped region should be default.
        assert_eq!(fb_pixels[0 * fb_w + 3], DEFAULT, "(3,0) should be default");
        assert_eq!(fb_pixels[3 * fb_w + 0], DEFAULT, "(0,3) should be default");
        assert_eq!(fb_pixels[9 * fb_w + 9], DEFAULT, "(9,9) should be default");
    }

    #[test]
    fn fill_rect_fully_offscreen() {
        const DEFAULT: u32 = 0xFF000000;

        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(DEFAULT);

        fill_rect(&mut fb, 20, 20, 5, 5, 0xFFFF0000);

        // Every pixel should remain default.
        for (i, &pixel) in fb.as_slice().iter().enumerate() {
            assert_eq!(pixel, DEFAULT, "pixel {i} should be default");
        }
    }

    #[test]
    fn fill_rect_alpha_blended() {
        const BLUE: u32 = 0xFF0000FF;
        const HALF_RED: u32 = 0x80FF0000; // alpha = 0x80 (128)

        let mut fb = Framebuffer::new(4, 4).unwrap();
        fb.clear(BLUE);

        fill_rect_alpha(&mut fb, 0, 0, 2, 2, HALF_RED);

        let fb_pixels = fb.as_slice();
        let fb_w = 4usize;

        // Check the 2x2 blended region.
        for y in 0..2u32 {
            for x in 0..2u32 {
                let idx = y as usize * fb_w + x as usize;
                let result = fb_pixels[idx];

                // R channel: src=255, dst=0 -> blended ~128
                let r = channel(result, 16);
                assert!(
                    r.abs_diff(128) <= 2,
                    "pixel ({x}, {y}) R should be ~128, got {r}"
                );

                // B channel: src=0, dst=255 -> blended ~127
                let b = channel(result, 0);
                assert!(
                    b.abs_diff(127) <= 2,
                    "pixel ({x}, {y}) B should be ~127, got {b}"
                );

                // A channel: always 0xFF for framebuffer output.
                let a = channel(result, 24);
                assert_eq!(a, 0xFF, "pixel ({x}, {y}) A should be 0xFF, got {a}");
            }
        }

        // Pixels outside the filled region should still be blue.
        assert_eq!(fb_pixels[0 * fb_w + 2], BLUE, "(2,0) should be blue");
        assert_eq!(fb_pixels[2 * fb_w + 0], BLUE, "(0,2) should be blue");
    }
}
