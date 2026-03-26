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
//! All three tiers share the same clipping logic via [`clip_blit`], which handles
//! negative destination coordinates, framebuffer bounds, and source texture bounds
//! before any pixels are touched.

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
pub(crate) fn clip_blit(
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
}
