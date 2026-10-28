#![allow(
    clippy::float_cmp,
    clippy::imprecise_flops,
    clippy::too_long_first_doc_paragraph
)]
//! Temporal anti-aliasing: reproject history, clamp or reject it, resolve.
//!
//! A renderer that draws one jittered sample per pixel per frame produces
//! an aliased, shimmering image. Temporal anti-aliasing (TAA) turns the
//! frame sequence itself into the multisample buffer: each frame's colour
//! is blended with a *reprojected* copy of the accumulated history, so a
//! static pixel effectively accumulates one new sub-pixel sample per
//! frame while the camera jitter ([`jitter_offset`]) walks the sample
//! position around inside the pixel.
//!
//! The resolve pass in this module is the part every temporal technique
//! (TAA, temporal upsampling, temporal GI denoisers) stands on, and it is
//! deliberately independent of any rasteriser:
//!
//! 1. **Reproject** ([`TemporalAccumulator::resolve`]): each pixel's
//!    motion vector says where its surface point was last frame, so the
//!    history buffer is bilinearly sampled at `pos - motion`.
//! 2. **Reject** history that cannot belong to the same surface:
//!    reprojection outside the frame, non-finite inputs, or a depth
//!    discontinuity against the stored previous depth (disocclusion —
//!    something new was uncovered, so the old colour is a different
//!    object and blending it in would ghost).
//! 3. **Clamp** surviving history into the colour AABB of the current
//!    frame's 3×3 neighbourhood (the Karis / Playdead neighbourhood
//!    clamp). History outside the range of colours the surface shows
//!    *now* is pulled to the box edge, which kills the colour ghosting
//!    that plain exponential blending leaves behind on lighting changes.
//! 4. **Blend** exponentially: `out = lerp(history, current, weight)`.
//!    The output becomes next frame's history, so a static pixel's
//!    effective sample count grows without bound while a moving pixel
//!    converges in a handful of frames.
//!
//! # Conventions
//!
//! * Buffers are row-major linear-space [`Color`] slices of exactly
//!   `width * height` elements. Pixel `(x, y)`'s centre is at continuous
//!   coordinate `(x, y)` for reprojection and bilinear sampling.
//! * A motion vector is in **pixels**, in the direction the surface
//!   moved: `motion = current_pos - previous_pos` (see
//!   [`motion_vector`]). The history sample position is therefore
//!   `current_pos - motion`.
//! * Depth follows the engine convention (smaller = nearer, matching
//!   [`crate::zbuffer::ZBuffer`]). The disocclusion test is relative:
//!   history is rejected when `|depth - prev_depth|` exceeds
//!   `depth_reject_relative * max(|depth|, |prev_depth|)`.
//! * History is stored as float colour, never quantised to the packed
//!   framebuffer format between frames — quantising history is a classic
//!   source of crawling and stuck pixels. The [`Framebuffer`] adapter
//!   ([`TemporalAccumulator::resolve_framebuffer`]) converts at the
//!   boundary only.
//!
//! # Invariants (the spec these functions are tested against)
//!
//! 1. The first frame after construction or [`TemporalAccumulator::reset`]
//!    is written through unchanged: there is no history to trust yet.
//! 2. A static scene with zero motion resolves to the current frame
//!    exactly, at any blend weight (history already equals current).
//! 3. A surface whose motion vectors track it exactly resolves to the
//!    current frame exactly — reprojection lands on its own previous
//!    colour, which the neighbourhood clamp leaves alone.
//! 4. Rejected history contributes nothing: a rejected pixel's output is
//!    exactly its (sanitised) current colour.
//! 5. Blended history never leaves the current frame's neighbourhood
//!    AABB: every output channel lies between the neighbourhood minimum
//!    and maximum of the current frame (the current pixel is part of its
//!    own neighbourhood, so the blend of two in-box values stays in-box).
//! 6. Every pixel is accounted for in [`TaaStats`]: it is either blended
//!    (`history_used`) or counted in exactly one rejection bucket.
//!
//! [`Framebuffer`]: crate::framebuffer::Framebuffer

use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::math::Vec2;

/// Parameters for one TAA resolve pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaaParams {
    /// Weight of the current frame in the blend, in `[0, 1]`.
    ///
    /// `out = history * (1 - w) + current * w`. Small values accumulate
    /// more history (smoother, slower to react); the classic default is
    /// 0.1 — roughly ten frames to converge. Sanitised in `resolve`:
    /// non-finite values fall back to the default, finite values are
    /// clamped to `[0, 1]`.
    pub current_weight: f32,
    /// Relative depth difference above which reprojected history is
    /// rejected as disoccluded. 0.1 rejects a 10% depth change.
    /// Sanitised in `resolve`: negative or non-finite values fall back
    /// to the default.
    pub depth_reject_relative: f32,
}

impl Default for TaaParams {
    fn default() -> Self {
        Self {
            current_weight: 0.1,
            depth_reject_relative: 0.1,
        }
    }
}

impl TaaParams {
    fn sanitised(self) -> Self {
        Self {
            current_weight: if self.current_weight.is_finite() {
                self.current_weight.clamp(0.0, 1.0)
            } else {
                0.1
            },
            depth_reject_relative: if self.depth_reject_relative.is_finite()
                && self.depth_reject_relative >= 0.0
            {
                self.depth_reject_relative
            } else {
                0.1
            },
        }
    }
}

/// Per-pass accounting returned by
/// [`TemporalAccumulator::resolve`]. Every pixel lands in exactly one of
/// `history_used` or a rejection bucket; `clamped_pixels` is a subset of
/// `history_used` (history that survived but had to be pulled into the
/// neighbourhood AABB before blending).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaaStats {
    /// Total pixels resolved (`width * height`).
    pub pixels: usize,
    /// Pixels blended with reprojected (and possibly clamped) history.
    pub history_used: usize,
    /// Pixels written through because no valid history exists yet
    /// (first frame after construction or `reset`).
    pub rejected_no_history: usize,
    /// Pixels whose reprojection fell outside the frame.
    pub rejected_out_of_bounds: usize,
    /// Pixels rejected by the depth-disocclusion test.
    pub rejected_depth: usize,
    /// Pixels rejected (or sanitised) because a motion vector, a depth
    /// value, or a colour was non-finite.
    pub rejected_non_finite: usize,
    /// Of `history_used`, how many had their history colour changed by
    /// the neighbourhood clamp.
    pub clamped_pixels: usize,
}

/// The motion vector of a surface point between two frames, in pixels:
/// `current - previous`.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::taa::motion_vector;
///
/// // The point moved 3 px right and 1 px up between frames.
/// let mv = motion_vector(Vec2::new(10.0, 10.0), Vec2::new(13.0, 9.0));
/// assert_eq!(mv, Vec2::new(3.0, -1.0));
/// ```
#[must_use]
pub fn motion_vector(previous_pos: Vec2, current_pos: Vec2) -> Vec2 {
    current_pos - previous_pos
}

/// Fill `out` with per-point motion vectors from paired previous/current
/// screen positions (for example, a vertex or feature tracked across two
/// frames).
///
/// # Panics
///
/// Panics if the three slices differ in length.
pub fn compute_motion_vectors(previous: &[Vec2], current: &[Vec2], out: &mut [Vec2]) {
    assert_eq!(
        previous.len(),
        current.len(),
        "position slice lengths differ"
    );
    assert_eq!(previous.len(), out.len(), "output slice length differs");
    for (o, (&p, &c)) in out.iter_mut().zip(previous.iter().zip(current)) {
        *o = motion_vector(p, c);
    }
}

/// One term of a Halton low-discrepancy sequence: the radical inverse of
/// `index` in `base`. `index` starts at 1 (index 0 is the origin for
/// every base). Returns 0 for a base below 2.
///
/// # Examples
///
/// ```
/// use abrash_core::taa::halton;
///
/// assert_eq!(halton(1, 2), 0.5);
/// assert_eq!(halton(2, 2), 0.25);
/// assert!((halton(1, 3) - 1.0 / 3.0).abs() < 1e-6);
/// ```
#[must_use]
pub fn halton(index: u32, base: u32) -> f32 {
    if base < 2 {
        return 0.0;
    }
    let mut f = 1.0f32;
    let mut r = 0.0f32;
    let mut i = index;
    let b = base as f32;
    while i > 0 {
        f /= b;
        r += f * (i % base) as f32;
        i /= base;
    }
    r
}

/// Sub-pixel camera jitter offset for `frame_index`, in pixels, with both
/// components in `(-0.5, 0.5)`.
///
/// Uses the Halton (2, 3) sequence — the standard TAA jitter: successive
/// frames sample a well-spread set of positions inside the pixel instead
/// of revisiting the same few offsets. A renderer applies this offset to
/// its projection, renders, then resolves with
/// [`TemporalAccumulator::resolve`]; static scenes accumulate the jittered
/// samples into a properly anti-aliased image.
///
/// # Examples
///
/// ```
/// use abrash_core::taa::jitter_offset;
///
/// let j = jitter_offset(0);
/// assert_eq!(j.x, 0.0); // halton(1, 2) - 0.5
/// assert!(j.x.abs() < 0.5 && j.y.abs() < 0.5);
/// ```
#[must_use]
pub fn jitter_offset(frame_index: u32) -> Vec2 {
    Vec2::new(
        halton(frame_index.wrapping_add(1), 2) - 0.5,
        halton(frame_index.wrapping_add(1), 3) - 0.5,
    )
}

/// A TAA history buffer plus the resolve pass that consumes it.
///
/// Owns the previous frame's resolved colours and depths. Feed it one
/// frame at a time, in order; call [`Self::reset`] on a scene cut, a
/// resize (construct a new accumulator instead), or a teleport — anywhere
/// last frame's image stops being a prediction of this frame's.
#[derive(Debug, Clone)]
pub struct TemporalAccumulator {
    width: u32,
    height: u32,
    history: Vec<Color>,
    prev_depth: Vec<f32>,
    has_history: bool,
    frame_index: u64,
}

impl TemporalAccumulator {
    /// Create an accumulator for `width` × `height` frames, with no
    /// history.
    ///
    /// # Errors
    ///
    /// Errors mirror [`Framebuffer::new`]: a dimension above `i32::MAX`
    /// or a pixel count overflowing `u32`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::taa::TemporalAccumulator;
    ///
    /// let acc = TemporalAccumulator::new(64, 64).unwrap();
    /// assert!(!acc.has_history());
    /// assert_eq!(acc.frame_index(), 0);
    /// ```
    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        if width > i32::MAX as u32 || height > i32::MAX as u32 {
            return Err("Buffer dimensions too large (max i32::MAX)");
        }
        let size = u64::from(width)
            .checked_mul(u64::from(height))
            .filter(|&s| u32::try_from(s).is_ok())
            .ok_or("Buffer size overflow")? as usize;
        Ok(Self {
            width,
            height,
            history: vec![Color::BLACK; size],
            prev_depth: vec![f32::INFINITY; size],
            has_history: false,
            frame_index: 0,
        })
    }

    /// Frame width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Frame height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Whether a previous resolved frame is available to reproject.
    #[must_use]
    pub const fn has_history(&self) -> bool {
        self.has_history
    }

    /// Number of frames resolved since construction or the last reset.
    #[must_use]
    pub const fn frame_index(&self) -> u64 {
        self.frame_index
    }

    /// The stored history colours (the last resolved output), row-major.
    #[must_use]
    pub fn history(&self) -> &[Color] {
        &self.history
    }

    /// Drop all history. The next resolve is a write-through first frame.
    pub const fn reset(&mut self) {
        self.has_history = false;
        self.frame_index = 0;
    }

    /// Resolve one frame: reproject, clamp/reject, blend, and store the
    /// result as the new history.
    ///
    /// All four slices must have exactly `width * height` elements;
    /// `output` receives the resolved colours.
    ///
    /// # Panics
    ///
    /// Panics on a length mismatch — a mismatched buffer is a
    /// programming error, not a runtime condition.
    pub fn resolve(
        &mut self,
        current: &[Color],
        motion: &[Vec2],
        depth: &[f32],
        params: &TaaParams,
        output: &mut [Color],
    ) -> TaaStats {
        let n = self.history.len();
        assert_eq!(current.len(), n, "current colour buffer length differs");
        assert_eq!(motion.len(), n, "motion buffer length differs");
        assert_eq!(depth.len(), n, "depth buffer length differs");
        assert_eq!(output.len(), n, "output buffer length differs");

        let params = params.sanitised();
        let w = self.width as usize;
        let h = self.height as usize;
        let mut stats = TaaStats {
            pixels: n,
            ..TaaStats::default()
        };

        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let cur_raw = current[i];
                let cur = sanitise(cur_raw);
                if !is_finite_color(cur_raw) {
                    stats.rejected_non_finite += 1;
                    output[i] = cur;
                    continue;
                }
                if !self.has_history {
                    stats.rejected_no_history += 1;
                    output[i] = cur;
                    continue;
                }
                let mv = motion[i];
                if !mv.x.is_finite() || !mv.y.is_finite() || !depth[i].is_finite() {
                    stats.rejected_non_finite += 1;
                    output[i] = cur;
                    continue;
                }
                // Reproject: where was this pixel's surface last frame?
                let px = x as f32 - mv.x;
                let py = y as f32 - mv.y;
                if px < 0.0 || py < 0.0 || px > (w - 1) as f32 || py > (h - 1) as f32 {
                    stats.rejected_out_of_bounds += 1;
                    output[i] = cur;
                    continue;
                }
                // Disocclusion: the surface now nearest at the
                // reprojected location must be at (nearly) the depth of
                // the surface now at this pixel, and vice versa — both
                // depths describe the same surface only then.
                let prev_depth = self.prev_depth[nearest_index(px, py, w)];
                if !prev_depth.is_finite() {
                    stats.rejected_non_finite += 1;
                    output[i] = cur;
                    continue;
                }
                let scale = depth[i].abs().max(prev_depth.abs()).max(1e-6);
                if (depth[i] - prev_depth).abs() > params.depth_reject_relative * scale {
                    stats.rejected_depth += 1;
                    output[i] = cur;
                    continue;
                }
                let hist_raw = sample_bilinear(&self.history, w, h, px, py);
                if !is_finite_color(hist_raw) {
                    stats.rejected_non_finite += 1;
                    output[i] = cur;
                    continue;
                }
                // Neighbourhood clamp: history may not claim a colour
                // the surface does not show anywhere nearby this frame.
                let (lo, hi) = neighbourhood_aabb(current, w, h, x, y);
                let hist = clamp_color(hist_raw, lo, hi);
                if hist != hist_raw {
                    stats.clamped_pixels += 1;
                }
                stats.history_used += 1;
                output[i] = hist.lerp(cur, params.current_weight);
            }
        }

        self.history.copy_from_slice(output);
        self.prev_depth.copy_from_slice(depth);
        self.has_history = true;
        self.frame_index += 1;
        stats
    }

    /// Resolve one frame held in packed framebuffers.
    ///
    /// Converts `current_fb` to linear colour, resolves against
    /// `motion`/`depth`, and writes the packed result to `out_fb`. All
    /// buffers must match this accumulator's dimensions.
    ///
    /// # Panics
    ///
    /// Panics if a framebuffer's dimensions differ from this
    /// accumulator's, or if `motion`/`depth` lengths mismatch (via
    /// [`Self::resolve`]). This is the
    /// integration path for renderers that already produce a
    /// [`Framebuffer`]; the float resolve in [`Self::resolve`] is the
    /// primitive.
    pub fn resolve_framebuffer(
        &mut self,
        current_fb: &Framebuffer,
        motion: &[Vec2],
        depth: &[f32],
        params: &TaaParams,
        out_fb: &mut Framebuffer,
    ) -> TaaStats {
        assert_eq!(
            current_fb.width(),
            self.width,
            "current framebuffer width differs"
        );
        assert_eq!(
            current_fb.height(),
            self.height,
            "current framebuffer height differs"
        );
        assert_eq!(
            out_fb.width(),
            self.width,
            "output framebuffer width differs"
        );
        assert_eq!(
            out_fb.height(),
            self.height,
            "output framebuffer height differs"
        );
        let current: Vec<Color> = current_fb
            .as_slice()
            .iter()
            .map(|&p| Color::from_argb_u32(p))
            .collect();
        let mut output = vec![Color::BLACK; current.len()];
        let stats = self.resolve(&current, motion, depth, params, &mut output);
        for (dst, src) in out_fb.as_mut_slice().iter_mut().zip(output.iter()) {
            *dst = src.to_argb_u32();
        }
        stats
    }
}

const fn is_finite_color(c: Color) -> bool {
    c.r.is_finite() && c.g.is_finite() && c.b.is_finite() && c.a.is_finite()
}

/// Replace non-finite channels with 0 so one bad pixel cannot poison the
/// float history for every frame that follows.
const fn sanitise(c: Color) -> Color {
    const fn f(v: f32) -> f32 {
        if v.is_finite() { v } else { 0.0 }
    }
    Color::new(f(c.r), f(c.g), f(c.b), f(c.a))
}

const fn nearest_index(px: f32, py: f32, w: usize) -> usize {
    let nx = px.round() as usize;
    let ny = py.round() as usize;
    ny * w + nx
}

/// Bilinear sample of a row-major colour buffer at continuous pixel
/// coordinates. The caller has already bounds-checked `(px, py)` to the
/// centre range `[0, w-1] × [0, h-1]`, so the footprint is in range.
fn sample_bilinear(buf: &[Color], w: usize, h: usize, px: f32, py: f32) -> Color {
    let x0 = px.floor() as usize;
    let y0 = py.floor() as usize;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let fx = px - x0 as f32;
    let fy = py - y0 as f32;
    let top = buf[y0 * w + x0].lerp(buf[y0 * w + x1], fx);
    let bottom = buf[y1 * w + x0].lerp(buf[y1 * w + x1], fx);
    top.lerp(bottom, fy)
}

/// Per-channel (RGB) min/max over the 3×3 neighbourhood of `(x, y)` in
/// the current frame, clipped at the frame edges.
fn neighbourhood_aabb(buf: &[Color], w: usize, h: usize, x: usize, y: usize) -> (Color, Color) {
    let mut lo = Color::new(f32::INFINITY, f32::INFINITY, f32::INFINITY, 0.0);
    let mut hi = Color::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY, 0.0);
    for dy in -1isize..=1 {
        for dx in -1isize..=1 {
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                continue;
            }
            let c = buf[ny as usize * w + nx as usize];
            // A non-finite neighbour must not widen the box to NaN.
            let c = sanitise(c);
            lo.r = lo.r.min(c.r);
            lo.g = lo.g.min(c.g);
            lo.b = lo.b.min(c.b);
            hi.r = hi.r.max(c.r);
            hi.g = hi.g.max(c.g);
            hi.b = hi.b.max(c.b);
        }
    }
    (lo, hi)
}

/// Clamp RGB into the neighbourhood box; alpha is left alone (the box is
/// a statement about surface colour, not coverage).
const fn clamp_color(c: Color, lo: Color, hi: Color) -> Color {
    Color::new(
        c.r.clamp(lo.r, hi.r),
        c.g.clamp(lo.g, hi.g),
        c.b.clamp(lo.b, hi.b),
        c.a,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(w: u32, h: u32) -> TemporalAccumulator {
        TemporalAccumulator::new(w, h).unwrap()
    }

    fn uniform(w: usize, h: usize, c: Color) -> Vec<Color> {
        vec![c; w * h]
    }

    fn zeros(n: usize) -> Vec<Vec2> {
        vec![Vec2::ZERO; n]
    }

    fn depths(n: usize, d: f32) -> Vec<f32> {
        vec![d; n]
    }

    fn resolve1(
        a: &mut TemporalAccumulator,
        cur: &[Color],
        mv: &[Vec2],
        d: &[f32],
    ) -> (Vec<Color>, TaaStats) {
        let mut out = vec![Color::BLACK; cur.len()];
        let s = a.resolve(cur, mv, d, &TaaParams::default(), &mut out);
        (out, s)
    }

    #[test]
    fn first_frame_writes_through_and_records_history() {
        let mut a = acc(4, 3);
        let cur = uniform(4, 3, Color::rgb(0.2, 0.4, 0.6));
        let (out, s) = resolve1(&mut a, &cur, &zeros(12), &depths(12, 0.5));
        assert_eq!(out, cur);
        assert_eq!(s.pixels, 12);
        assert_eq!(s.rejected_no_history, 12);
        assert_eq!(s.history_used, 0);
        assert!(a.has_history());
        assert_eq!(a.frame_index(), 1);
        assert_eq!(a.history(), cur.as_slice());
    }

    #[test]
    fn static_scene_resolves_to_current_exactly() {
        let mut a = acc(8, 8);
        // A varied scene: neighbourhood boxes are wide, but history
        // already equals current, so no blend error can appear.
        let cur: Vec<Color> = (0..64)
            .map(|i| Color::rgb(i as f32 / 64.0, 0.3, 1.0 - i as f32 / 64.0))
            .collect();
        resolve1(&mut a, &cur, &zeros(64), &depths(64, 0.5));
        let (out, s) = resolve1(&mut a, &cur, &zeros(64), &depths(64, 0.5));
        assert_eq!(out, cur);
        assert_eq!(s.history_used, 64);
        assert_eq!(s.clamped_pixels, 0);
    }

    #[test]
    fn blend_is_exponential_with_current_weight() {
        // 3x3: frame 1 all black; frame 2 centre white, neighbours black,
        // so the centre's neighbourhood box is [0, 1] and history (0)
        // survives the clamp. Output centre = 0.9*0 + 0.1*1 = 0.1.
        let mut a = acc(3, 3);
        resolve1(
            &mut a,
            &uniform(3, 3, Color::BLACK),
            &zeros(9),
            &depths(9, 0.5),
        );
        let mut cur = uniform(3, 3, Color::BLACK);
        cur[4] = Color::WHITE;
        let (out, s) = resolve1(&mut a, &cur, &zeros(9), &depths(9, 0.5));
        assert!((out[4].r - 0.1).abs() < 1e-6, "got {}", out[4].r);
        assert_eq!(s.history_used, 9);
        // Corners see an all-black neighbourhood in frame 2 as well.
        assert_eq!(out[0], Color::BLACK);
    }

    #[test]
    fn tracked_motion_resolves_to_current_exactly() {
        // Content shifts one pixel right; motion (1, 0) reprojects each
        // pixel onto its own previous colour.
        let (w, h) = (8usize, 2usize);
        let mut a = acc(8, 2);
        let frame1: Vec<Color> = (0..w * h)
            .map(|i| Color::grey((i % w) as f32 / 8.0))
            .collect();
        resolve1(&mut a, &frame1, &zeros(w * h), &depths(w * h, 0.5));
        let mut frame2 = frame1.clone();
        for y in 0..h {
            for x in 1..w {
                frame2[y * w + x] = frame1[y * w + x - 1];
            }
            frame2[y * w] = Color::BLACK; // newly entered column
        }
        let mv = vec![Vec2::new(1.0, 0.0); w * h];
        let (out, s) = resolve1(&mut a, &frame2, &mv, &depths(w * h, 0.5));
        // Column 0 reprojects outside the frame and writes through; the
        // rest land exactly on their tracked colour.
        assert_eq!(out, frame2);
        assert_eq!(s.rejected_out_of_bounds, h);
        assert_eq!(s.history_used, w * h - h);
    }

    #[test]
    fn untracked_motion_ghosts_but_tracked_does_not() {
        // Same shift as above but with zero motion vectors the resolve
        // must NOT reproduce frame 2 — it blends stale history. This is
        // the ghost TAA motion vectors exist to prevent.
        let (w, h) = (8usize, 1usize);
        let mut a = acc(8, 1);
        let frame1: Vec<Color> = (0..w).map(|i| Color::grey(i as f32 / 8.0)).collect();
        resolve1(&mut a, &frame1, &zeros(w), &depths(w, 0.5));
        let mut frame2 = frame1.clone();
        for x in 1..w {
            frame2[x] = frame1[x - 1];
        }
        let (out, _) = resolve1(&mut a, &frame2, &zeros(w), &depths(w, 0.5));
        assert_ne!(out[4], frame2[4]);
    }

    #[test]
    fn depth_disocclusion_rejects_history() {
        let mut a = acc(4, 4);
        resolve1(
            &mut a,
            &uniform(4, 4, Color::RED),
            &zeros(16),
            &depths(16, 0.5),
        );
        // A nearer surface (depth 0.5 -> 0.2 is a 60% relative change)
        // covers the frame: every pixel is new, nothing may blend.
        let (out, s) = resolve1(
            &mut a,
            &uniform(4, 4, Color::BLUE),
            &zeros(16),
            &depths(16, 0.2),
        );
        assert_eq!(out, uniform(4, 4, Color::BLUE));
        assert_eq!(s.rejected_depth, 16);
        assert_eq!(s.history_used, 0);
    }

    #[test]
    fn small_depth_drift_keeps_history() {
        let mut a = acc(2, 2);
        resolve1(
            &mut a,
            &uniform(2, 2, Color::grey(0.4)),
            &zeros(4),
            &depths(4, 0.50),
        );
        // 0.50 -> 0.52 is a 4% relative change, inside the 10% band.
        let (_, s) = resolve1(
            &mut a,
            &uniform(2, 2, Color::grey(0.4)),
            &zeros(4),
            &depths(4, 0.52),
        );
        assert_eq!(s.history_used, 4);
        assert_eq!(s.rejected_depth, 0);
    }

    #[test]
    fn neighbourhood_clamp_kills_colour_ghost() {
        // Lighting change: a red surface turns blue in one frame, zero
        // motion, same depth. The blue neighbourhood box contains no
        // red, so history is clamped to blue and the output is exactly
        // blue — no purple ghost frame.
        let mut a = acc(4, 4);
        resolve1(
            &mut a,
            &uniform(4, 4, Color::RED),
            &zeros(16),
            &depths(16, 0.5),
        );
        let (out, s) = resolve1(
            &mut a,
            &uniform(4, 4, Color::BLUE),
            &zeros(16),
            &depths(16, 0.5),
        );
        assert_eq!(out, uniform(4, 4, Color::BLUE));
        assert_eq!(s.history_used, 16);
        assert_eq!(s.clamped_pixels, 16);
    }

    #[test]
    fn output_stays_inside_current_neighbourhood_box() {
        // Invariant 5 under adversarial history: random-ish frames, the
        // output channel values must stay within the global current
        // range (the neighbourhood box is a subset of it).
        let mut a = acc(5, 5);
        let f1: Vec<Color> = (0..25)
            .map(|i| Color::rgb((i * 37 % 10) as f32 / 10.0, 0.9, 0.1))
            .collect();
        let f2: Vec<Color> = (0..25)
            .map(|i| Color::rgb(0.2, (i * 53 % 10) as f32 / 10.0, 0.3))
            .collect();
        resolve1(&mut a, &f1, &zeros(25), &depths(25, 0.5));
        let (out, _) = resolve1(&mut a, &f2, &zeros(25), &depths(25, 0.5));
        for c in &out {
            assert!((0.0..=1.0).contains(&c.r));
            assert!(c.g >= 0.0 && c.g <= 0.9 + 1e-6);
        }
    }

    #[test]
    fn out_of_bounds_reprojection_rejects() {
        let mut a = acc(4, 4);
        resolve1(
            &mut a,
            &uniform(4, 4, Color::RED),
            &zeros(16),
            &depths(16, 0.5),
        );
        let mv = vec![Vec2::new(100.0, 0.0); 16];
        let (out, s) = resolve1(&mut a, &uniform(4, 4, Color::GREEN), &mv, &depths(16, 0.5));
        assert_eq!(out, uniform(4, 4, Color::GREEN));
        assert_eq!(s.rejected_out_of_bounds, 16);
    }

    #[test]
    fn non_finite_motion_and_depth_reject_and_bad_colour_is_sanitised() {
        let mut a = acc(2, 1);
        resolve1(
            &mut a,
            &uniform(2, 1, Color::RED),
            &zeros(2),
            &depths(2, 0.5),
        );
        let mut mv = zeros(2);
        mv[0] = Vec2::new(f32::NAN, 0.0);
        let (out, s) = resolve1(&mut a, &uniform(2, 1, Color::GREEN), &mv, &depths(2, 0.5));
        assert_eq!(s.rejected_non_finite, 1);
        assert_eq!(out[0], Color::GREEN); // pixel 0 wrote through
        assert_eq!(s.history_used, 1); // pixel 1 blended normally

        // A NaN current colour is sanitised to 0 in that channel and
        // does not poison the stored history.
        let mut bad = uniform(2, 1, Color::GREEN);
        bad[1] = Color::new(f32::NAN, 0.5, 0.0, 1.0);
        let (out, s) = resolve1(&mut a, &bad, &zeros(2), &depths(2, 0.5));
        assert_eq!(s.rejected_non_finite, 1);
        assert_eq!(out[1].r, 0.0);
        assert!(is_finite_color(a.history()[1]));
    }

    #[test]
    fn half_pixel_motion_bilinearly_samples_history() {
        // 2x1 history [0, 1] grey; motion (0.5, 0) at pixel 1 samples
        // history at x=0.5 -> 0.5 grey. Current frame is built so the
        // clamp box at pixel 1 is [0, 1] and the blend is observable:
        // out = 0.9 * 0.5 + 0.1 * current.
        let mut a = acc(2, 1);
        let f1 = vec![Color::BLACK, Color::WHITE];
        resolve1(&mut a, &f1, &zeros(2), &depths(2, 0.5));
        let f2 = vec![Color::BLACK, Color::WHITE];
        let mv = vec![Vec2::ZERO, Vec2::new(0.5, 0.0)];
        let (out, _) = resolve1(&mut a, &f2, &mv, &depths(2, 0.5));
        assert!((out[1].r - 0.55).abs() < 1e-6, "got {}", out[1].r);
    }

    #[test]
    fn reset_restores_first_frame_behaviour() {
        let mut a = acc(2, 2);
        resolve1(
            &mut a,
            &uniform(2, 2, Color::RED),
            &zeros(4),
            &depths(4, 0.5),
        );
        a.reset();
        assert!(!a.has_history());
        assert_eq!(a.frame_index(), 0);
        let (_, s) = resolve1(
            &mut a,
            &uniform(2, 2, Color::BLUE),
            &zeros(4),
            &depths(4, 0.5),
        );
        assert_eq!(s.rejected_no_history, 4);
    }

    #[test]
    fn every_pixel_is_accounted_for() {
        // Mixed scene: some tracked, some disoccluded, some off-frame.
        let mut a = acc(4, 1);
        resolve1(
            &mut a,
            &uniform(4, 1, Color::RED),
            &zeros(4),
            &depths(4, 0.5),
        );
        let mv = vec![
            Vec2::ZERO,                    // blends
            Vec2::new(9.0, 0.0),           // out of bounds
            Vec2::ZERO,                    // depth will reject
            Vec2::new(f32::INFINITY, 0.0), // non-finite
        ];
        let d = vec![0.5, 0.5, 0.1, 0.5];
        let (_, s) = resolve1(&mut a, &uniform(4, 1, Color::GREEN), &mv, &d);
        assert_eq!(
            s.history_used
                + s.rejected_no_history
                + s.rejected_out_of_bounds
                + s.rejected_depth
                + s.rejected_non_finite,
            s.pixels
        );
        assert_eq!(s.history_used, 1);
        assert_eq!(s.rejected_out_of_bounds, 1);
        assert_eq!(s.rejected_depth, 1);
        assert_eq!(s.rejected_non_finite, 1);
    }

    #[test]
    fn flicker_is_damped_over_frames() {
        // A lone pixel strobing black/white: its own neighbourhood box
        // is just itself, so the clamp passes current through — TAA
        // cannot (and should not) smooth a 1-pixel strobe. In a 3x3
        // patch strobing together the same holds per-pixel. What TAA
        // does damp is sub-box jitter: alternate between two greys
        // inside a varied neighbourhood and the output variance shrinks.
        let mut a = acc(3, 3);
        let mut base = uniform(3, 3, Color::grey(0.5));
        base[4] = Color::grey(0.4);
        resolve1(&mut a, &base, &zeros(9), &depths(9, 0.5));
        let mut prev_out = 0.4f32;
        let mut deltas = Vec::new();
        for frame in 0..8 {
            let v = if frame % 2 == 0 { 0.6 } else { 0.4 };
            let mut cur = uniform(3, 3, Color::grey(0.5));
            cur[4] = Color::grey(v);
            // Neighbours span [0, 1] via two fixed corner anchors so the
            // centre's box never collapses onto the strobing value.
            cur[0] = Color::BLACK;
            cur[8] = Color::WHITE;
            let (out, _) = resolve1(&mut a, &cur, &zeros(9), &depths(9, 0.5));
            deltas.push((out[4].r - prev_out).abs());
            prev_out = out[4].r;
        }
        // Input strobe amplitude is 0.2 per frame; the resolved centre
        // must move strictly less than that once history accumulates.
        assert!(deltas[4] < 0.2, "delta {} not damped", deltas[4]);
        assert!(deltas[4] < deltas[0] + 1e-6);
    }

    #[test]
    fn halton_matches_known_values_and_stays_in_unit_interval() {
        assert_eq!(halton(1, 2), 0.5);
        assert_eq!(halton(2, 2), 0.25);
        assert_eq!(halton(3, 2), 0.75);
        assert!((halton(2, 3) - 2.0 / 3.0).abs() < 1e-6);
        assert_eq!(halton(5, 1), 0.0);
        assert_eq!(halton(0, 2), 0.0);
        for i in 1..64 {
            let v = halton(i, 2);
            assert!((0.0..1.0).contains(&v));
        }
    }

    #[test]
    fn jitter_offsets_stay_inside_the_pixel_and_vary() {
        let mut seen = Vec::new();
        for f in 0..16 {
            let j = jitter_offset(f);
            assert!(j.x.abs() < 0.5 && j.y.abs() < 0.5);
            assert!(!seen.contains(&j), "jitter repeated at frame {f}");
            seen.push(j);
        }
    }

    #[test]
    fn motion_vector_helpers_agree() {
        let prev = vec![Vec2::new(1.0, 2.0), Vec2::new(5.0, 5.0)];
        let cur = vec![Vec2::new(4.0, 2.0), Vec2::new(5.0, 1.0)];
        let mut out = vec![Vec2::ZERO; 2];
        compute_motion_vectors(&prev, &cur, &mut out);
        assert_eq!(out, vec![Vec2::new(3.0, 0.0), Vec2::new(0.0, -4.0)]);
    }

    #[test]
    fn custom_current_weight_is_respected_and_sanitised() {
        let mut a = acc(3, 3);
        resolve1(
            &mut a,
            &uniform(3, 3, Color::BLACK),
            &zeros(9),
            &depths(9, 0.5),
        );
        let mut cur = uniform(3, 3, Color::BLACK);
        cur[4] = Color::WHITE;
        let mut out = vec![Color::BLACK; 9];
        // Weight 1.0 = no history at all.
        let p = TaaParams {
            current_weight: 1.0,
            depth_reject_relative: 0.1,
        };
        a.resolve(&cur, &zeros(9), &depths(9, 0.5), &p, &mut out);
        assert_eq!(out[4], Color::WHITE);
        // NaN weight falls back to the 0.1 default instead of NaN output.
        let p = TaaParams {
            current_weight: f32::NAN,
            depth_reject_relative: 0.1,
        };
        let mut out2 = vec![Color::BLACK; 9];
        a.resolve(&cur, &zeros(9), &depths(9, 0.5), &p, &mut out2);
        assert!(out2[4].r.is_finite());
    }

    #[test]
    fn framebuffer_adapter_round_trips_a_static_scene() {
        let mut a = acc(4, 4);
        let mut fb = Framebuffer::new(4, 4).unwrap();
        fb.clear(0xFF20_40_60);
        let mut out_fb = Framebuffer::new(4, 4).unwrap();
        let s1 = a.resolve_framebuffer(
            &fb,
            &zeros(16),
            &depths(16, 0.5),
            &TaaParams::default(),
            &mut out_fb,
        );
        assert_eq!(s1.rejected_no_history, 16);
        assert_eq!(out_fb.as_slice(), fb.as_slice());
        let s2 = a.resolve_framebuffer(
            &fb,
            &zeros(16),
            &depths(16, 0.5),
            &TaaParams::default(),
            &mut out_fb,
        );
        assert_eq!(s2.history_used, 16);
        assert_eq!(out_fb.as_slice(), fb.as_slice());
    }

    #[test]
    #[should_panic(expected = "motion buffer length differs")]
    fn mismatched_buffers_panic() {
        let mut a = acc(2, 2);
        let mut out = vec![Color::BLACK; 4];
        a.resolve(
            &uniform(2, 2, Color::BLACK),
            &zeros(3),
            &depths(4, 0.5),
            &TaaParams::default(),
            &mut out,
        );
    }

    #[test]
    fn invalid_dimensions_error() {
        assert!(TemporalAccumulator::new(u32::MAX, 2).is_err());
    }
}
