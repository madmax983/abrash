# 2D Blitter Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a fast, general-purpose 2D bitmap blitter to abrash-core with three tiers: opaque (memcpy), color-key (branch), alpha (SWAR blend).

**Architecture:** Single new module `crates/abrash-core/src/blitter.rs` with free functions operating on `Framebuffer` + `Texture`. Clip-once-then-raw-loop pattern: safe variants compute clipped region, then delegate to unchecked inner loops. No new dependencies.

**Tech Stack:** Rust, abrash-core (Framebuffer, Texture, blend_swar from texture.rs)

---

### Task 1: Module scaffold + SrcRect type + clip_blit()

**Files:**
- Create: `crates/abrash-core/src/blitter.rs`
- Modify: `crates/abrash-core/src/lib.rs:11-28` (add `pub mod blitter;`)

**Step 1: Write the failing test for clip_blit**

In `crates/abrash-core/src/blitter.rs`, create the module with tests:

```rust
//! Fast 2D bitmap blitting.
//!
//! Three tiers of blit, from fastest to most capable:
//! - **Opaque**: `copy_from_slice` per row (memcpy speed)
//! - **Color-key**: skip pixels matching a key color (one branch per pixel)
//! - **Alpha**: per-pixel SWAR alpha blend with 0x00/0xFF fast paths

/// Source rectangle within a texture atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrcRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Result of clipping a blit against framebuffer and texture bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClippedBlit {
    src_x: u32,
    src_y: u32,
    dst_x: u32,
    dst_y: u32,
    w: u32,
    h: u32,
}

/// Clip a blit operation against framebuffer bounds and texture bounds.
///
/// Returns `None` if the blit is fully off-screen or the source rect is empty.
fn clip_blit(
    src: &SrcRect,
    dst_x: i32,
    dst_y: i32,
    fb_w: u32,
    fb_h: u32,
    tex_w: u32,
    tex_h: u32,
) -> Option<ClippedBlit> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- clip_blit tests ---

    #[test]
    fn clip_blit_no_clipping_needed() {
        let src = SrcRect { x: 0, y: 0, w: 16, h: 16 };
        let result = clip_blit(&src, 10, 10, 800, 600, 64, 64);
        assert_eq!(result, Some(ClippedBlit {
            src_x: 0, src_y: 0, dst_x: 10, dst_y: 10, w: 16, h: 16,
        }));
    }

    #[test]
    fn clip_blit_left_overflow() {
        let src = SrcRect { x: 0, y: 0, w: 16, h: 16 };
        let result = clip_blit(&src, -4, 10, 800, 600, 64, 64);
        assert_eq!(result, Some(ClippedBlit {
            src_x: 4, src_y: 0, dst_x: 0, dst_y: 10, w: 12, h: 16,
        }));
    }

    #[test]
    fn clip_blit_top_overflow() {
        let src = SrcRect { x: 0, y: 0, w: 16, h: 16 };
        let result = clip_blit(&src, 10, -6, 800, 600, 64, 64);
        assert_eq!(result, Some(ClippedBlit {
            src_x: 0, src_y: 6, dst_x: 10, dst_y: 0, w: 16, h: 10,
        }));
    }

    #[test]
    fn clip_blit_right_overflow() {
        let src = SrcRect { x: 0, y: 0, w: 16, h: 16 };
        let result = clip_blit(&src, 790, 10, 800, 600, 64, 64);
        assert_eq!(result, Some(ClippedBlit {
            src_x: 0, src_y: 0, dst_x: 790, dst_y: 10, w: 10, h: 16,
        }));
    }

    #[test]
    fn clip_blit_bottom_overflow() {
        let src = SrcRect { x: 0, y: 0, w: 16, h: 16 };
        let result = clip_blit(&src, 10, 592, 800, 600, 64, 64);
        assert_eq!(result, Some(ClippedBlit {
            src_x: 0, src_y: 0, dst_x: 10, dst_y: 592, w: 16, h: 8,
        }));
    }

    #[test]
    fn clip_blit_fully_offscreen_left() {
        let src = SrcRect { x: 0, y: 0, w: 16, h: 16 };
        assert_eq!(clip_blit(&src, -20, 10, 800, 600, 64, 64), None);
    }

    #[test]
    fn clip_blit_fully_offscreen_right() {
        let src = SrcRect { x: 0, y: 0, w: 16, h: 16 };
        assert_eq!(clip_blit(&src, 800, 10, 800, 600, 64, 64), None);
    }

    #[test]
    fn clip_blit_fully_offscreen_top() {
        let src = SrcRect { x: 0, y: 0, w: 16, h: 16 };
        assert_eq!(clip_blit(&src, 10, -20, 800, 600, 64, 64), None);
    }

    #[test]
    fn clip_blit_fully_offscreen_bottom() {
        let src = SrcRect { x: 0, y: 0, w: 16, h: 16 };
        assert_eq!(clip_blit(&src, 10, 600, 800, 600, 64, 64), None);
    }

    #[test]
    fn clip_blit_zero_size_src() {
        let src = SrcRect { x: 0, y: 0, w: 0, h: 16 };
        assert_eq!(clip_blit(&src, 10, 10, 800, 600, 64, 64), None);
        let src = SrcRect { x: 0, y: 0, w: 16, h: 0 };
        assert_eq!(clip_blit(&src, 10, 10, 800, 600, 64, 64), None);
    }

    #[test]
    fn clip_blit_src_exceeds_texture() {
        // SrcRect extends beyond texture — clamp to texture bounds
        let src = SrcRect { x: 60, y: 60, w: 16, h: 16 };
        let result = clip_blit(&src, 10, 10, 800, 600, 64, 64);
        assert_eq!(result, Some(ClippedBlit {
            src_x: 60, src_y: 60, dst_x: 10, dst_y: 10, w: 4, h: 4,
        }));
    }

    #[test]
    fn clip_blit_src_fully_outside_texture() {
        let src = SrcRect { x: 64, y: 0, w: 16, h: 16 };
        assert_eq!(clip_blit(&src, 10, 10, 800, 600, 64, 64), None);
    }

    #[test]
    fn clip_blit_corner_diagonal() {
        // Sprite straddles top-left corner
        let src = SrcRect { x: 0, y: 0, w: 32, h: 32 };
        let result = clip_blit(&src, -8, -8, 800, 600, 64, 64);
        assert_eq!(result, Some(ClippedBlit {
            src_x: 8, src_y: 8, dst_x: 0, dst_y: 0, w: 24, h: 24,
        }));
    }

    #[test]
    fn clip_blit_exact_fit() {
        // Sprite fills entire framebuffer exactly
        let src = SrcRect { x: 0, y: 0, w: 100, h: 100 };
        let result = clip_blit(&src, 0, 0, 100, 100, 100, 100);
        assert_eq!(result, Some(ClippedBlit {
            src_x: 0, src_y: 0, dst_x: 0, dst_y: 0, w: 100, h: 100,
        }));
    }

    #[test]
    fn clip_blit_1x1_sprite() {
        let src = SrcRect { x: 5, y: 5, w: 1, h: 1 };
        let result = clip_blit(&src, 50, 50, 800, 600, 64, 64);
        assert_eq!(result, Some(ClippedBlit {
            src_x: 5, src_y: 5, dst_x: 50, dst_y: 50, w: 1, h: 1,
        }));
    }

    #[test]
    fn clip_blit_atlas_subregion() {
        // Sprite from middle of atlas, partially offscreen right
        let src = SrcRect { x: 32, y: 16, w: 16, h: 16 };
        let result = clip_blit(&src, 795, 100, 800, 600, 64, 64);
        assert_eq!(result, Some(ClippedBlit {
            src_x: 32, src_y: 16, dst_x: 795, dst_y: 100, w: 5, h: 16,
        }));
    }
}
```

**Step 2: Register the module**

In `crates/abrash-core/src/lib.rs`, add after line 28:

```rust
pub mod blitter;
```

**Step 3: Run tests to verify they fail**

Run: `cargo test -p abrash-core blitter -- --nocapture`
Expected: FAIL — `todo!()` panics

**Step 4: Implement clip_blit**

Replace the `todo!()` in `clip_blit` with:

```rust
fn clip_blit(
    src: &SrcRect,
    dst_x: i32,
    dst_y: i32,
    fb_w: u32,
    fb_h: u32,
    tex_w: u32,
    tex_h: u32,
) -> Option<ClippedBlit> {
    if src.w == 0 || src.h == 0 {
        return None;
    }

    // Clamp source rect to texture bounds
    let tex_max_w = tex_w.saturating_sub(src.x);
    let tex_max_h = tex_h.saturating_sub(src.y);
    if tex_max_w == 0 || tex_max_h == 0 {
        return None;
    }
    let mut w = src.w.min(tex_max_w);
    let mut h = src.h.min(tex_max_h);

    let mut src_x = src.x;
    let mut src_y = src.y;
    let mut dx = dst_x;
    let mut dy = dst_y;

    // Left overflow: dst_x < 0
    if dx < 0 {
        let skip = (-dx) as u32;
        if skip >= w {
            return None;
        }
        src_x += skip;
        w -= skip;
        dx = 0;
    }

    // Top overflow: dst_y < 0
    if dy < 0 {
        let skip = (-dy) as u32;
        if skip >= h {
            return None;
        }
        src_y += skip;
        h -= skip;
        dy = 0;
    }

    let dx = dx as u32;
    let dy = dy as u32;

    // Right overflow
    if dx >= fb_w {
        return None;
    }
    let max_w = fb_w - dx;
    w = w.min(max_w);

    // Bottom overflow
    if dy >= fb_h {
        return None;
    }
    let max_h = fb_h - dy;
    h = h.min(max_h);

    if w == 0 || h == 0 {
        return None;
    }

    Some(ClippedBlit {
        src_x,
        src_y,
        dst_x: dx,
        dst_y: dy,
        w,
        h,
    })
}
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p abrash-core blitter -- --nocapture`
Expected: ALL PASS (16 clip tests)

**Step 6: Commit**

```bash
git add crates/abrash-core/src/blitter.rs crates/abrash-core/src/lib.rs
git commit -m "feat(blitter): add SrcRect type and clip_blit() with 16 clipping tests"
```

---

### Task 2: blit_opaque + blit_opaque_unchecked

**Files:**
- Modify: `crates/abrash-core/src/blitter.rs`

**Step 1: Write the failing tests**

Add to the `tests` module in `blitter.rs`:

```rust
    use crate::framebuffer::Framebuffer;
    use crate::texture::Texture;

    /// Helper: create a texture with a known checkerboard pattern.
    fn make_checkerboard_texture(w: u32, h: u32) -> Texture {
        let mut tex = Texture::new(w, h).unwrap();
        for y in 0..h {
            for x in 0..w {
                let color = if (x + y) % 2 == 0 { 0xFFFF0000 } else { 0xFF00FF00 };
                tex.set_pixel(x, y, color);
            }
        }
        tex
    }

    // --- blit_opaque tests ---

    #[test]
    fn blit_opaque_basic() {
        let tex = make_checkerboard_texture(4, 4);
        let mut fb = Framebuffer::new(16, 16).unwrap();
        let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };

        blit_opaque(&mut fb, &tex, src, 2, 3);

        // Verify pixels were copied
        for y in 0..4u32 {
            for x in 0..4u32 {
                let expected = if (x + y) % 2 == 0 { 0xFFFF0000 } else { 0xFF00FF00 };
                assert_eq!(
                    fb.get_pixel((x + 2) as i32, (y + 3) as i32),
                    Some(expected),
                    "Mismatch at dst ({}, {})", x + 2, y + 3,
                );
            }
        }

        // Verify surrounding pixels are untouched
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF00_0000));
        assert_eq!(fb.get_pixel(1, 3), Some(0xFF00_0000));
    }

    #[test]
    fn blit_opaque_clipped_left() {
        let tex = make_checkerboard_texture(8, 8);
        let mut fb = Framebuffer::new(16, 16).unwrap();
        let src = SrcRect { x: 0, y: 0, w: 8, h: 8 };

        blit_opaque(&mut fb, &tex, src, -3, 0);

        // First 5 columns of sprite should be visible (src_x 3..8 -> dst_x 0..5)
        for y in 0..8u32 {
            for x in 0..5u32 {
                let src_x = x + 3;
                let expected = if (src_x + y) % 2 == 0 { 0xFFFF0000 } else { 0xFF00FF00 };
                assert_eq!(
                    fb.get_pixel(x as i32, y as i32),
                    Some(expected),
                    "Mismatch at dst ({}, {})", x, y,
                );
            }
        }
    }

    #[test]
    fn blit_opaque_fully_offscreen() {
        let tex = make_checkerboard_texture(4, 4);
        let mut fb = Framebuffer::new(16, 16).unwrap();
        let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };

        // Should be a no-op
        blit_opaque(&mut fb, &tex, src, -10, 0);

        // All pixels still default
        for y in 0..16 {
            for x in 0..16 {
                assert_eq!(fb.get_pixel(x, y), Some(0xFF00_0000));
            }
        }
    }

    #[test]
    fn blit_opaque_atlas_subregion() {
        let tex = make_checkerboard_texture(16, 16);
        let mut fb = Framebuffer::new(32, 32).unwrap();
        // Blit a 4x4 region from the middle of the texture
        let src = SrcRect { x: 4, y: 4, w: 4, h: 4 };

        blit_opaque(&mut fb, &tex, src, 10, 10);

        for y in 0..4u32 {
            for x in 0..4u32 {
                let src_x = x + 4;
                let src_y = y + 4;
                let expected = if (src_x + src_y) % 2 == 0 { 0xFFFF0000 } else { 0xFF00FF00 };
                assert_eq!(
                    fb.get_pixel((x + 10) as i32, (y + 10) as i32),
                    Some(expected),
                    "Mismatch at dst ({}, {})", x + 10, y + 10,
                );
            }
        }
    }

    #[test]
    fn blit_opaque_unchecked_basic() {
        let tex = make_checkerboard_texture(4, 4);
        let mut fb = Framebuffer::new(16, 16).unwrap();
        let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };

        // Safety: 4x4 at (2,3) fits within 16x16
        unsafe { blit_opaque_unchecked(&mut fb, &tex, src, 2, 3); }

        for y in 0..4u32 {
            for x in 0..4u32 {
                let expected = if (x + y) % 2 == 0 { 0xFFFF0000 } else { 0xFF00FF00 };
                assert_eq!(
                    fb.get_pixel((x + 2) as i32, (y + 3) as i32),
                    Some(expected),
                );
            }
        }
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-core blitter -- --nocapture`
Expected: FAIL — functions don't exist

**Step 3: Implement blit_opaque_unchecked and blit_opaque**

Add to `blitter.rs` (before the `tests` module):

```rust
use crate::framebuffer::Framebuffer;
use crate::texture::Texture;

/// Blit a rectangular region from a texture to the framebuffer, fully opaque.
///
/// Uses `copy_from_slice` per row — compiles to memcpy. This is the fastest
/// possible blit: pure memory bandwidth.
///
/// Destination coordinates are signed to support partially off-screen sprites.
/// Automatically clips against framebuffer and texture bounds.
pub fn blit_opaque(fb: &mut Framebuffer, tex: &Texture, src: SrcRect, dst_x: i32, dst_y: i32) {
    let Some(c) = clip_blit(&src, dst_x, dst_y, fb.width(), fb.height(), tex.width, tex.height)
    else {
        return;
    };
    // Safety: clip_blit guarantees all coordinates are in-bounds.
    unsafe { blit_opaque_unchecked(fb, tex, SrcRect { x: c.src_x, y: c.src_y, w: c.w, h: c.h }, c.dst_x, c.dst_y); }
}

/// Blit a rectangular region from a texture to the framebuffer, fully opaque.
///
/// # Safety
///
/// Caller must guarantee:
/// - `src.x + src.w <= tex.width` and `src.y + src.h <= tex.height`
/// - `dst_x + src.w <= fb.width()` and `dst_y + src.h <= fb.height()`
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-core blitter -- --nocapture`
Expected: ALL PASS

**Step 5: Commit**

```bash
git add crates/abrash-core/src/blitter.rs
git commit -m "feat(blitter): add blit_opaque and blit_opaque_unchecked (memcpy per row)"
```

---

### Task 3: blit_colorkey + blit_colorkey_unchecked

**Files:**
- Modify: `crates/abrash-core/src/blitter.rs`

**Step 1: Write the failing tests**

Add to the `tests` module:

```rust
    // --- blit_colorkey tests ---

    #[test]
    fn blit_colorkey_skips_key_color() {
        let mut tex = Texture::new(4, 4).unwrap();
        let key = 0xFFFF00FF; // magenta
        // Fill: top-left 2x2 = red, rest = magenta (transparent)
        for y in 0..4 {
            for x in 0..4 {
                let color = if x < 2 && y < 2 { 0xFFFF0000 } else { key };
                tex.set_pixel(x, y, color);
            }
        }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(0xFF0000FF); // blue background
        let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };

        blit_colorkey(&mut fb, &tex, src, 5, 5, key);

        // Red pixels should overwrite blue
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(fb.get_pixel(x + 5, y + 5), Some(0xFFFF0000),
                    "Should be red at ({}, {})", x + 5, y + 5);
            }
        }
        // Magenta pixels should leave blue intact
        assert_eq!(fb.get_pixel(7, 5), Some(0xFF0000FF));
        assert_eq!(fb.get_pixel(5, 7), Some(0xFF0000FF));
        assert_eq!(fb.get_pixel(8, 8), Some(0xFF0000FF));
    }

    #[test]
    fn blit_colorkey_all_transparent() {
        let key = 0xFFFF00FF;
        let mut tex = Texture::new(4, 4).unwrap();
        for y in 0..4 {
            for x in 0..4 {
                tex.set_pixel(x, y, key);
            }
        }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(0xFF0000FF);
        let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };

        blit_colorkey(&mut fb, &tex, src, 5, 5, key);

        // All blue — nothing overwritten
        for y in 5..9 {
            for x in 5..9 {
                assert_eq!(fb.get_pixel(x, y), Some(0xFF0000FF));
            }
        }
    }

    #[test]
    fn blit_colorkey_clipped() {
        let mut tex = Texture::new(8, 8).unwrap();
        let key = 0xFFFF00FF;
        for y in 0..8 {
            for x in 0..8 {
                tex.set_pixel(x, y, 0xFFFF0000); // all red (no transparency)
            }
        }
        let mut fb = Framebuffer::new(16, 16).unwrap();
        let src = SrcRect { x: 0, y: 0, w: 8, h: 8 };

        blit_colorkey(&mut fb, &tex, src, -3, 0, key);

        // Columns 0..5 should be red, column 5+ should be default
        assert_eq!(fb.get_pixel(0, 0), Some(0xFFFF0000));
        assert_eq!(fb.get_pixel(4, 0), Some(0xFFFF0000));
        assert_eq!(fb.get_pixel(5, 0), Some(0xFF00_0000)); // untouched
    }

    #[test]
    fn blit_colorkey_unchecked_basic() {
        let mut tex = Texture::new(4, 4).unwrap();
        let key = 0xFFFF00FF;
        for y in 0..4 {
            for x in 0..4 {
                let color = if x == 0 { 0xFFFF0000 } else { key };
                tex.set_pixel(x, y, color);
            }
        }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(0xFF0000FF);
        let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };

        unsafe { blit_colorkey_unchecked(&mut fb, &tex, src, 0, 0, key); }

        // First column red, rest blue
        for y in 0..4 {
            assert_eq!(fb.get_pixel(0, y), Some(0xFFFF0000));
            assert_eq!(fb.get_pixel(1, y), Some(0xFF0000FF));
        }
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-core blitter -- --nocapture`
Expected: FAIL — functions don't exist

**Step 3: Implement blit_colorkey_unchecked and blit_colorkey**

Add to `blitter.rs`:

```rust
/// Blit with color-key transparency. Pixels matching `key` are skipped.
///
/// Automatically clips against framebuffer and texture bounds.
pub fn blit_colorkey(
    fb: &mut Framebuffer,
    tex: &Texture,
    src: SrcRect,
    dst_x: i32,
    dst_y: i32,
    key: u32,
) {
    let Some(c) = clip_blit(&src, dst_x, dst_y, fb.width(), fb.height(), tex.width, tex.height)
    else {
        return;
    };
    unsafe {
        blit_colorkey_unchecked(
            fb, tex,
            SrcRect { x: c.src_x, y: c.src_y, w: c.w, h: c.h },
            c.dst_x, c.dst_y, key,
        );
    }
}

/// Blit with color-key transparency, no bounds checking.
///
/// # Safety
///
/// Same requirements as `blit_opaque_unchecked`.
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-core blitter -- --nocapture`
Expected: ALL PASS

**Step 5: Commit**

```bash
git add crates/abrash-core/src/blitter.rs
git commit -m "feat(blitter): add blit_colorkey and blit_colorkey_unchecked"
```

---

### Task 4: blit_alpha + blit_alpha_unchecked

**Files:**
- Modify: `crates/abrash-core/src/blitter.rs`

**Step 1: Write the failing tests**

Add to the `tests` module:

```rust
    // --- blit_alpha tests ---

    /// Helper: extract channel from 0xAARRGGBB
    fn channel(color: u32, shift: u32) -> u32 {
        (color >> shift) & 0xFF
    }

    #[test]
    fn blit_alpha_fully_opaque() {
        let mut tex = Texture::new(4, 4).unwrap();
        for y in 0..4 { for x in 0..4 { tex.set_pixel(x, y, 0xFFFF0000); } }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(0xFF0000FF); // blue
        let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };

        blit_alpha(&mut fb, &tex, src, 5, 5);

        // Fully opaque red should overwrite blue
        for y in 5..9 {
            for x in 5..9 {
                assert_eq!(fb.get_pixel(x, y), Some(0xFFFF0000));
            }
        }
    }

    #[test]
    fn blit_alpha_fully_transparent() {
        let mut tex = Texture::new(4, 4).unwrap();
        for y in 0..4 { for x in 0..4 { tex.set_pixel(x, y, 0x00FF0000); } } // alpha=0

        let mut fb = Framebuffer::new(16, 16).unwrap();
        fb.clear(0xFF0000FF); // blue
        let src = SrcRect { x: 0, y: 0, w: 4, h: 4 };

        blit_alpha(&mut fb, &tex, src, 5, 5);

        // Transparent — blue should remain
        for y in 5..9 {
            for x in 5..9 {
                assert_eq!(fb.get_pixel(x, y), Some(0xFF0000FF));
            }
        }
    }

    #[test]
    fn blit_alpha_50_percent() {
        let mut tex = Texture::new(1, 1).unwrap();
        tex.set_pixel(0, 0, 0x80FF0000); // 50% alpha red

        let mut fb = Framebuffer::new(4, 4).unwrap();
        fb.clear(0xFF0000FF); // blue
        let src = SrcRect { x: 0, y: 0, w: 1, h: 1 };

        blit_alpha(&mut fb, &tex, src, 0, 0);

        let result = fb.get_pixel(0, 0).unwrap();
        // With alpha=0x80 (~128/255):
        // R: 255*128/255 + 0*(255-128)/255 ≈ 128
        // B: 0*128/255 + 255*(255-128)/255 ≈ 127
        // Allow +/- 2 for rounding
        let r = channel(result, 16);
        let b = channel(result, 0);
        assert!(r >= 126 && r <= 130, "R channel: expected ~128, got {r}");
        assert!(b >= 125 && b <= 129, "B channel: expected ~127, got {b}");
    }

    #[test]
    fn blit_alpha_clipped() {
        let mut tex = Texture::new(8, 8).unwrap();
        for y in 0..8 { for x in 0..8 { tex.set_pixel(x, y, 0xFFFF0000); } }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        let src = SrcRect { x: 0, y: 0, w: 8, h: 8 };

        blit_alpha(&mut fb, &tex, src, -3, -3);

        assert_eq!(fb.get_pixel(0, 0), Some(0xFFFF0000));
        assert_eq!(fb.get_pixel(4, 4), Some(0xFFFF0000));
        assert_eq!(fb.get_pixel(5, 0), Some(0xFF00_0000)); // untouched
    }

    #[test]
    fn blit_alpha_unchecked_basic() {
        let mut tex = Texture::new(2, 2).unwrap();
        for y in 0..2 { for x in 0..2 { tex.set_pixel(x, y, 0xFFFF0000); } }

        let mut fb = Framebuffer::new(16, 16).unwrap();
        let src = SrcRect { x: 0, y: 0, w: 2, h: 2 };

        unsafe { blit_alpha_unchecked(&mut fb, &tex, src, 0, 0); }

        assert_eq!(fb.get_pixel(0, 0), Some(0xFFFF0000));
        assert_eq!(fb.get_pixel(1, 1), Some(0xFFFF0000));
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-core blitter -- --nocapture`
Expected: FAIL — functions don't exist

**Step 3: Implement alpha_blend_pixel, blit_alpha_unchecked, and blit_alpha**

Add to `blitter.rs`:

```rust
/// Alpha-blend a single source pixel over a destination pixel (src-over compositing).
///
/// Uses SWAR to blend R+B and G+A channels in parallel.
/// Formula: `out = src * alpha + dst * (255 - alpha)` per channel.
#[inline(always)]
fn alpha_blend_pixel(src: u32, dst: u32) -> u32 {
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

/// Blit with per-pixel alpha blending (src-over compositing).
///
/// Fast paths: 0xFF alpha → direct copy, 0x00 alpha → skip.
/// Automatically clips against framebuffer and texture bounds.
pub fn blit_alpha(fb: &mut Framebuffer, tex: &Texture, src: SrcRect, dst_x: i32, dst_y: i32) {
    let Some(c) = clip_blit(&src, dst_x, dst_y, fb.width(), fb.height(), tex.width, tex.height)
    else {
        return;
    };
    unsafe {
        blit_alpha_unchecked(
            fb, tex,
            SrcRect { x: c.src_x, y: c.src_y, w: c.w, h: c.h },
            c.dst_x, c.dst_y,
        );
    }
}

/// Blit with per-pixel alpha blending, no bounds checking.
///
/// # Safety
///
/// Same requirements as `blit_opaque_unchecked`.
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
                // Fully opaque — direct copy
                fb_pixels[dst_row_start + col] = src_px;
            } else if alpha > 0 {
                // Partial alpha — SWAR blend
                fb_pixels[dst_row_start + col] =
                    alpha_blend_pixel(src_px, fb_pixels[dst_row_start + col]);
            }
            // alpha == 0 → skip (fully transparent)
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p abrash-core blitter -- --nocapture`
Expected: ALL PASS

**Step 5: Commit**

```bash
git add crates/abrash-core/src/blitter.rs
git commit -m "feat(blitter): add blit_alpha and blit_alpha_unchecked with SWAR blending"
```

---

### Task 5: Re-export from abrash crate + fill_rect utility

**Files:**
- Modify: `crates/abrash-core/src/blitter.rs` (add fill_rect)
- Modify: `abrash/src/lib.rs` (add re-export)

**Step 1: Write the failing tests**

Add to `tests` module in `blitter.rs`:

```rust
    // --- fill_rect tests ---

    #[test]
    fn fill_rect_basic() {
        let mut fb = Framebuffer::new(16, 16).unwrap();
        fill_rect(&mut fb, 2, 3, 4, 5, 0xFFFF0000);

        for y in 0..16 {
            for x in 0..16 {
                let expected = if x >= 2 && x < 6 && y >= 3 && y < 8 {
                    0xFFFF0000
                } else {
                    0xFF00_0000
                };
                assert_eq!(fb.get_pixel(x, y), Some(expected), "Mismatch at ({x}, {y})");
            }
        }
    }

    #[test]
    fn fill_rect_clipped_negative() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fill_rect(&mut fb, -2, -2, 5, 5, 0xFFFF0000);

        for y in 0..10 {
            for x in 0..10 {
                let expected = if x < 3 && y < 3 { 0xFFFF0000 } else { 0xFF00_0000 };
                assert_eq!(fb.get_pixel(x, y), Some(expected), "Mismatch at ({x}, {y})");
            }
        }
    }

    #[test]
    fn fill_rect_fully_offscreen() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fill_rect(&mut fb, 20, 20, 5, 5, 0xFFFF0000);

        for y in 0..10 {
            for x in 0..10 {
                assert_eq!(fb.get_pixel(x, y), Some(0xFF00_0000));
            }
        }
    }

    #[test]
    fn fill_rect_alpha_blended() {
        let mut fb = Framebuffer::new(4, 4).unwrap();
        fb.clear(0xFF0000FF); // blue
        fill_rect_alpha(&mut fb, 0, 0, 2, 2, 0x80FF0000); // 50% red

        let result = fb.get_pixel(0, 0).unwrap();
        let r = channel(result, 16);
        let b = channel(result, 0);
        assert!(r >= 126 && r <= 130, "R: expected ~128, got {r}");
        assert!(b >= 125 && b <= 129, "B: expected ~127, got {b}");
    }
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p abrash-core blitter -- --nocapture`
Expected: FAIL — functions don't exist

**Step 3: Implement fill_rect and fill_rect_alpha**

Add to `blitter.rs`:

```rust
/// Fill a rectangle with a solid color.
///
/// Like `Framebuffer::clear_rect` but uses signed coordinates for consistency
/// with the blit API. Clips automatically.
pub fn fill_rect(fb: &mut Framebuffer, x: i32, y: i32, w: u32, h: u32, color: u32) {
    fb.clear_rect(x, y, w, h, color);
}

/// Fill a rectangle with an alpha-blended color.
///
/// Each pixel in the rectangle is blended: `out = color * alpha + dst * (255 - alpha)`.
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
    let x1 = ((x as i64 + w as i64).min(fb_w as i64) as i32).max(0) as u32;
    let y1 = ((y as i64 + h as i64).min(fb_h as i64) as i32).max(0) as u32;

    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let stride = fb.width() as usize;
    let fb_pixels = fb.as_mut_slice();

    for row in y0..y1 {
        let row_start = row as usize * stride;
        for col in x0..x1 {
            let idx = row_start + col as usize;
            fb_pixels[idx] = alpha_blend_pixel(color, fb_pixels[idx]);
        }
    }
}
```

**Step 4: Add re-export to abrash/src/lib.rs**

After the existing `pub use abrash_core::...` block, add:

```rust
pub use abrash_core::blitter;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p abrash-core blitter -- --nocapture`
Expected: ALL PASS

**Step 6: Commit**

```bash
git add crates/abrash-core/src/blitter.rs abrash/src/lib.rs
git commit -m "feat(blitter): add fill_rect, fill_rect_alpha, and re-export from abrash crate"
```

---

### Task 6: cargo fmt + clippy + full test suite

**Files:**
- All modified files

**Step 1: Format**

Run: `cargo fmt --all`

**Step 2: Clippy**

Run: `cargo clippy -p abrash-core -- -W clippy::pedantic`
Fix any warnings.

**Step 3: Full test suite**

Run: `cargo test -p abrash-core`
Expected: ALL existing + new tests pass

**Step 4: Commit (if fmt/clippy changes)**

```bash
git add -A
git commit -m "chore: fmt + clippy fixes for blitter module"
```

---

### Task 7: Criterion benchmarks

**Files:**
- Create: `crates/abrash-core/benches/blitter.rs`
- Modify: `crates/abrash-core/Cargo.toml` (add bench target if needed)

**Step 1: Check if criterion is already a dev-dependency**

Run: `grep criterion crates/abrash-core/Cargo.toml`

If not present, add to `[dev-dependencies]`:
```toml
criterion = { version = "0.5", features = ["html_reports"] }
```

And add bench target:
```toml
[[bench]]
name = "blitter"
harness = false
```

**Step 2: Create benchmark file**

```rust
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use abrash_core::blitter::{blit_opaque, blit_colorkey, blit_alpha, SrcRect};
use abrash_core::framebuffer::Framebuffer;
use abrash_core::texture::Texture;

fn make_opaque_texture(w: u32, h: u32) -> Texture {
    let mut tex = Texture::new(w, h).unwrap();
    for y in 0..h {
        for x in 0..w {
            tex.set_pixel(x, y, 0xFFFF0000);
        }
    }
    tex
}

fn make_alpha_texture(w: u32, h: u32, alpha: u8) -> Texture {
    let mut tex = Texture::new(w, h).unwrap();
    let color = (alpha as u32) << 24 | 0x00FF0000;
    for y in 0..h {
        for x in 0..w {
            tex.set_pixel(x, y, color);
        }
    }
    tex
}

fn bench_blit_opaque(c: &mut Criterion) {
    let mut group = c.benchmark_group("blit_opaque");
    for size in [16u32, 32, 64, 128] {
        let pixels = (size * size) as u64;
        group.throughput(Throughput::Elements(pixels));
        let tex = make_opaque_texture(size, size);
        let mut fb = Framebuffer::new(1024, 768).unwrap();
        let src = SrcRect { x: 0, y: 0, w: size, h: size };

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                blit_opaque(black_box(&mut fb), black_box(&tex), src, 100, 100);
            });
        });
    }
    group.finish();
}

fn bench_blit_colorkey(c: &mut Criterion) {
    let mut group = c.benchmark_group("blit_colorkey");
    for size in [16u32, 32, 64, 128] {
        let pixels = (size * size) as u64;
        group.throughput(Throughput::Elements(pixels));
        let tex = make_opaque_texture(size, size);
        let mut fb = Framebuffer::new(1024, 768).unwrap();
        let src = SrcRect { x: 0, y: 0, w: size, h: size };

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                blit_colorkey(black_box(&mut fb), black_box(&tex), src, 100, 100, 0xFFFF00FF);
            });
        });
    }
    group.finish();
}

fn bench_blit_alpha(c: &mut Criterion) {
    let mut group = c.benchmark_group("blit_alpha");
    for size in [16u32, 32, 64, 128] {
        let pixels = (size * size) as u64;
        group.throughput(Throughput::Elements(pixels));
        let tex = make_alpha_texture(size, size, 0x80);
        let mut fb = Framebuffer::new(1024, 768).unwrap();
        let src = SrcRect { x: 0, y: 0, w: size, h: size };

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                blit_alpha(black_box(&mut fb), black_box(&tex), src, 100, 100);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_blit_opaque, bench_blit_colorkey, bench_blit_alpha);
criterion_main!(benches);
```

**Step 3: Run benchmarks**

Run: `cargo bench -p abrash-core --bench blitter`
Expected: All three groups produce results. Record megapixels/sec for each size.

**Step 4: Commit**

```bash
git add crates/abrash-core/benches/blitter.rs crates/abrash-core/Cargo.toml
git commit -m "bench(blitter): add criterion benchmarks for all three blit tiers"
```
