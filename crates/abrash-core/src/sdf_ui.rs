#![allow(clippy::float_cmp, clippy::imprecise_flops)]
//! Analytic 2D SDF coverage for UI and text — MSAA-quality edges with no
//! multisample buffer.
//!
//! GPU text/UI shaders (the classic Valve / Greenworks SDF idiom) do not
//! rasterise a hard edge. They evaluate an exact signed distance `d` per
//! pixel and convert it to *coverage* with the screen-space derivative of
//! the distance:
//!
//! ```glsl
//! float w = fwidth(d);
//! float alpha = smoothstep(w, -w, d); // 1 inside, 0 outside, AA at the edge
//! ```
//!
//! This module is the CPU equivalent for Abrash's software rasteriser:
//!
//! - [`smoothstep`] — GLSL-style Hermite interpolation, reversed edges
//!   included (GLSL leaves `edge0 >= edge1` undefined; the SDF idiom
//!   depends on it, so here it is defined).
//! - [`fwidth_from_samples`] — `fwidth(d) ≈ |d(p+ex) − d(p)| + |d(p+ey) − d(p)|`,
//!   the same Manhattan approximation GPUs use (`abs(dFdx) + abs(dFdy)`).
//! - [`sdf_coverage`] — distance → coverage in `[0, 1]`.
//! - [`sample_coverage`] / [`sample_coverage_scaled`] — evaluate an SDF and
//!   its two neighbour taps, then derive `fwidth` and coverage. The scaled
//!   form takes the world-space size of one pixel, so the same shape (or
//!   glyph outline) stays correctly anti-aliased at any UI scale — that is
//!   what makes the coverage resolution-independent.
//! - [`fill_sdf_shape`] / [`stroke_sdf_shape`] — render any closure over the
//!   existing 2D primitives in [`crate::sdf`] into a [`Framebuffer`],
//!   blending with the computed coverage. No multisample buffer, no
//!   supersampling: three distance taps per pixel, and only inside the
//!   caller's clip rectangle.
//!
//! # Invariants (the spec these functions are tested against)
//!
//! 1. Coverage is always in `[0, 1]`, is exactly `0.5` at `d == 0` when
//!    `fwidth > 0`, is `1` for `d <= -fwidth / 2`, `0` for
//!    `d >= fwidth / 2`, and is monotonically non-increasing in `d`.
//! 2. `fwidth` is non-negative, and is `≈ 1` pixel-unit for a true SDF
//!    (gradient magnitude 1) sampled in pixel space.
//! 3. Rendering never touches a pixel outside the clip rectangle, never
//!    changes a pixel whose coverage is `0`, and fully replaces a pixel
//!    whose coverage is `1` with an opaque colour.
//!
//! Pixel space for the renderers is the framebuffer's own space: `x` right,
//! `y` down, pixel centres at `(x + 0.5, y + 0.5)`. Compose shapes with the
//! primitives and CSG helpers in [`crate::sdf`].

use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::irect::IRect;
use crate::math::Vec2;

/// GLSL-style smoothstep: Hermite interpolation between `edge0` and `edge1`.
///
/// Returns `0` at/below `edge0`, `1` at/above `edge1`, with a smooth
/// (zero-derivative) ramp between them. Unlike GLSL, reversed edges are
/// defined: `smoothstep(1, -1, x)` is the decreasing ramp the SDF coverage
/// idiom needs. Equal edges degenerate to a step at that edge (returning
/// `0.5` exactly on it).
///
/// # Examples
///
/// ```
/// use abrash_core::sdf_ui::smoothstep;
///
/// assert_eq!(smoothstep(0.0, 1.0, 0.5), 0.5);
/// assert_eq!(smoothstep(0.0, 1.0, -1.0), 0.0);
/// // The reversed-edge form used for SDF coverage:
/// assert_eq!(smoothstep(0.5, -0.5, 0.0), 0.5);
/// ```
#[must_use]
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if edge0 == edge1 {
        return if x < edge0 {
            0.0
        } else if x > edge0 {
            1.0
        } else {
            0.5
        };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Screen-space derivative magnitude of a distance field, from samples.
///
/// `fwidth(d) = |d_x - d| + |d_y - d|` where `d_x` / `d_y` are the distances
/// one pixel to the right / below. This is the Manhattan form GPUs compute
/// with `abs(dFdx(d)) + abs(dFdy(d))`.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf_ui::fwidth_from_samples;
///
/// // A true SDF in pixel space changes by ~1 across one pixel.
/// assert!((fwidth_from_samples(-2.0, -1.0, -1.0) - 2.0).abs() < 1e-6);
/// // A constant field has zero width.
/// assert_eq!(fwidth_from_samples(3.0, 3.0, 3.0), 0.0);
/// ```
#[must_use]
#[inline]
pub fn fwidth_from_samples(d: f32, d_x: f32, d_y: f32) -> f32 {
    (d_x - d).abs() + (d_y - d).abs()
}

/// Convert a signed distance to pixel coverage in `[0, 1]`.
///
/// This is `smoothstep(fwidth / 2, -fwidth / 2, d)`: full coverage at least
/// half a derivative-width inside the shape, zero coverage at least half a
/// width outside, and a smooth ramp across the edge. A non-positive
/// `fwidth` (constant field, degenerate sample) falls back to a hard step,
/// with `d == 0` counting as half covered.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf_ui::sdf_coverage;
///
/// assert_eq!(sdf_coverage(0.0, 1.0), 0.5); // on the edge
/// assert_eq!(sdf_coverage(-1.0, 1.0), 1.0); // well inside
/// assert_eq!(sdf_coverage(1.0, 1.0), 0.0); // well outside
/// ```
#[must_use]
#[inline]
pub fn sdf_coverage(d: f32, fwidth: f32) -> f32 {
    if !(fwidth > 0.0) || !d.is_finite() {
        return if d < 0.0 {
            1.0
        } else if d > 0.0 {
            0.0
        } else {
            0.5
        };
    }
    smoothstep(0.5 * fwidth, -0.5 * fwidth, d)
}

/// Coverage of a stroke centred on the `d == 0` contour.
///
/// The stroke occupies `|d| <= half_width`; its coverage is the fill
/// coverage of the band distance `|d| - half_width`, so both stroke edges
/// are anti-aliased the same way as a fill edge.
///
/// # Examples
///
/// ```
/// use abrash_core::sdf_ui::stroke_coverage;
///
/// assert_eq!(stroke_coverage(0.0, 1.0, 2.0), 1.0); // on the contour
/// assert_eq!(stroke_coverage(2.0, 1.0, 2.0), 0.5); // on the stroke edge
/// assert_eq!(stroke_coverage(4.0, 1.0, 2.0), 0.0); // clear of the stroke
/// ```
#[must_use]
#[inline]
pub fn stroke_coverage(d: f32, fwidth: f32, half_width: f32) -> f32 {
    sdf_coverage(d.abs() - half_width.max(0.0), fwidth)
}

/// One pixel's distance sample: the distance, its screen-space derivative
/// width, and the resulting coverage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoverageSample {
    /// Signed distance at the pixel centre (negative inside).
    pub distance: f32,
    /// Estimated `fwidth(distance)` in the same units as `distance`.
    pub fwidth: f32,
    /// Fill coverage in `[0, 1]` derived from `distance` and `fwidth`.
    pub coverage: f32,
}

/// Sample fill coverage for `sdf` at pixel-space point `p`.
///
/// Evaluates `sdf` at `p`, `p + (1, 0)`, and `p + (0, 1)` — the same three
/// taps a shader gets from `dFdx` / `dFdy` — and derives `fwidth` and
/// coverage from them.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::circle_2d;
/// use abrash_core::sdf_ui::sample_coverage;
///
/// let shape = |p: Vec2| circle_2d(p, Vec2::new(10.0, 10.0), 5.0);
/// assert_eq!(sample_coverage(shape, Vec2::new(10.0, 10.0)).coverage, 1.0);
/// assert_eq!(sample_coverage(shape, Vec2::new(30.0, 10.0)).coverage, 0.0);
/// ```
#[must_use]
#[inline]
pub fn sample_coverage(sdf: impl Fn(Vec2) -> f32, p: Vec2) -> CoverageSample {
    sample_coverage_scaled(sdf, p, Vec2::ONE)
}

/// Sample fill coverage where one pixel spans `pixel_step` world units.
///
/// `p` and the SDF are in world (e.g. UI layout or glyph) units;
/// `pixel_step` is how far one screen pixel moves in that space — the
/// world-space analogue of evaluating neighbours one pixel away. Scaling
/// the UI down (`pixel_step` grows) widens the coverage ramp in world
/// units by exactly the same factor, keeping edges one pixel wide on
/// screen at every scale.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::circle_2d;
/// use abrash_core::sdf_ui::sample_coverage_scaled;
///
/// let shape = |p: Vec2| circle_2d(p, Vec2::ZERO, 1.0);
/// // On the unit circle's edge, coverage is 0.5 at any pixel scale.
/// let s = sample_coverage_scaled(shape, Vec2::new(1.0, 0.0), Vec2::splat(0.01));
/// assert!((s.coverage - 0.5).abs() < 1e-6, "coverage={}", s.coverage);
/// assert!((s.fwidth - 0.01005).abs() < 1e-4, "fwidth={}", s.fwidth);
/// ```
#[must_use]
pub fn sample_coverage_scaled(
    sdf: impl Fn(Vec2) -> f32,
    p: Vec2,
    pixel_step: Vec2,
) -> CoverageSample {
    let distance = sdf(p);
    let d_x = sdf(p + Vec2::new(pixel_step.x, 0.0));
    let d_y = sdf(p + Vec2::new(0.0, pixel_step.y));
    let fwidth = fwidth_from_samples(distance, d_x, d_y);
    CoverageSample {
        distance,
        fwidth,
        coverage: sdf_coverage(distance, fwidth),
    }
}

/// Blend `color` over the framebuffer pixel at `(x, y)` with `coverage`.
///
/// Coverage `0` leaves the pixel byte-identical (no lossy sRGB round-trip);
/// coverage `1` with an opaque colour writes the colour directly.
fn blend_pixel(fb: &mut Framebuffer, x: i32, y: i32, color: Color, coverage: f32) {
    if coverage <= 0.0 {
        return;
    }
    if coverage >= 1.0 && color.a >= 1.0 {
        fb.set_pixel(x, y, color.to_argb_u32());
        return;
    }
    let Some(dst_argb) = fb.get_pixel(x, y) else {
        return;
    };
    let src = color.with_alpha(color.a.clamp(0.0, 1.0) * coverage);
    let out = Color::blend_over(src, Color::from_argb_u32(dst_argb));
    fb.set_pixel(x, y, out.to_argb_u32());
}

/// Shared raster loop for fill and stroke rendering.
fn render_coverage(
    fb: &mut Framebuffer,
    clip: Option<IRect>,
    color: Color,
    stroke_half_width: Option<f32>,
    sdf: &impl Fn(Vec2) -> f32,
) {
    let screen = IRect::from_size(fb.width() as i32, fb.height() as i32);
    let Some(region) = clip.map_or(Some(screen), |c| screen.intersect(c)) else {
        return;
    };
    for y in region.min.y..region.max.y {
        for x in region.min.x..region.max.x {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let sample = sample_coverage(sdf, p);
            let coverage = stroke_half_width.map_or(sample.coverage, |half| {
                stroke_coverage(sample.distance, sample.fwidth, half)
            });
            blend_pixel(fb, x, y, color, coverage);
        }
    }
}

/// Fill an analytic 2D SDF shape into `fb` with per-pixel coverage.
///
/// `sdf` is evaluated in framebuffer pixel space (pixel centres at
/// half-integers). Only pixels inside `clip` (or the whole framebuffer when
/// `clip` is `None`) are considered. Each covered pixel is composited with
/// [`Color::blend_over`] at alpha `color.a * coverage`, giving MSAA-quality
/// edges from a single analytic distance per pixel — no multisample or
/// supersampled buffer is allocated.
///
/// # Examples
///
/// ```
/// use abrash_core::color::Color;
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::rounded_rect_2d;
/// use abrash_core::sdf_ui::fill_sdf_shape;
///
/// let mut fb = Framebuffer::new(40, 24).unwrap();
/// fill_sdf_shape(&mut fb, None, Color::WHITE, |p| {
///     rounded_rect_2d(p, Vec2::new(20.0, 12.0), Vec2::new(14.0, 8.0), 4.0)
/// });
/// // Centre is fully covered; a far corner is untouched (still black).
/// assert_eq!(fb.get_pixel(20, 12), Some(Color::WHITE.to_argb_u32()));
/// assert_eq!(fb.get_pixel(0, 0), Some(0xFF00_0000));
/// ```
pub fn fill_sdf_shape(
    fb: &mut Framebuffer,
    clip: Option<IRect>,
    color: Color,
    sdf: impl Fn(Vec2) -> f32,
) {
    render_coverage(fb, clip, color, None, &sdf);
}

/// Stroke the `d == 0` contour of an analytic 2D SDF shape into `fb`.
///
/// The stroke extends `half_width` pixels to either side of the contour
/// and is anti-aliased on both edges with the same coverage rule as
/// [`fill_sdf_shape`]. Useful for panel borders, focus rings, icon
/// outlines, and glyph-like strokes without a separate outline mesh.
///
/// # Examples
///
/// ```
/// use abrash_core::color::Color;
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_core::math::Vec2;
/// use abrash_core::sdf::circle_2d;
/// use abrash_core::sdf_ui::stroke_sdf_shape;
///
/// let mut fb = Framebuffer::new(32, 32).unwrap();
/// stroke_sdf_shape(&mut fb, None, Color::WHITE, 1.0, |p| {
///     circle_2d(p, Vec2::new(16.0, 16.0), 10.0)
/// });
/// // On the ring: painted. At the circle's centre: untouched.
/// assert_ne!(fb.get_pixel(26, 16), Some(0xFF00_0000));
/// assert_eq!(fb.get_pixel(16, 16), Some(0xFF00_0000));
/// ```
pub fn stroke_sdf_shape(
    fb: &mut Framebuffer,
    clip: Option<IRect>,
    color: Color,
    half_width: f32,
    sdf: impl Fn(Vec2) -> f32,
) {
    render_coverage(fb, clip, color, Some(half_width.max(0.0)), &sdf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ivec::IVec2;
    use crate::sdf;

    // ── Spec: smoothstep ─────────────────────────────────────────────────

    #[test]
    fn smoothstep_endpoints_and_midpoint() {
        assert_eq!(smoothstep(0.0, 1.0, 0.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 1.0), 1.0);
        assert_eq!(smoothstep(0.0, 1.0, 0.5), 0.5);
        assert_eq!(smoothstep(0.0, 1.0, -3.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 3.0), 1.0);
    }

    #[test]
    fn smoothstep_reversed_edges_mirror_the_ramp() {
        for x in [-1.0_f32, -0.25, 0.0, 0.25, 1.0] {
            let fwd = smoothstep(-1.0, 1.0, x);
            let rev = smoothstep(1.0, -1.0, x);
            assert!((fwd + rev - 1.0).abs() < 1e-6, "x={x} fwd={fwd} rev={rev}");
        }
    }

    #[test]
    fn smoothstep_degenerate_edge_is_a_step() {
        assert_eq!(smoothstep(2.0, 2.0, 1.0), 0.0);
        assert_eq!(smoothstep(2.0, 2.0, 2.0), 0.5);
        assert_eq!(smoothstep(2.0, 2.0, 3.0), 1.0);
    }

    // ── Spec: coverage invariants ────────────────────────────────────────

    #[test]
    fn coverage_is_half_on_the_edge_and_saturates_half_width_away() {
        assert_eq!(sdf_coverage(0.0, 2.0), 0.5);
        assert_eq!(sdf_coverage(-1.0, 2.0), 1.0);
        assert_eq!(sdf_coverage(1.0, 2.0), 0.0);
        assert_eq!(sdf_coverage(-100.0, 2.0), 1.0);
        assert_eq!(sdf_coverage(100.0, 2.0), 0.0);
    }

    #[test]
    fn coverage_is_monotonically_non_increasing_in_d() {
        let mut prev = f32::INFINITY;
        for i in -20..=20 {
            let c = sdf_coverage(i as f32 * 0.25, 1.5);
            assert!((0.0..=1.0).contains(&c), "coverage {c} out of range");
            assert!(c <= prev + 1e-7, "coverage rose at step {i}: {prev} -> {c}");
            prev = c;
        }
    }

    #[test]
    fn coverage_zero_fwidth_falls_back_to_a_step() {
        assert_eq!(sdf_coverage(-0.001, 0.0), 1.0);
        assert_eq!(sdf_coverage(0.0, 0.0), 0.5);
        assert_eq!(sdf_coverage(0.001, 0.0), 0.0);
        assert_eq!(sdf_coverage(-1.0, -2.0), 1.0);
    }

    #[test]
    fn coverage_scales_with_fwidth() {
        // The same distance is "more outside" when the pixel footprint is
        // small, and "barely outside" when it is large: resolution
        // independence in one equation.
        let tight = sdf_coverage(0.4, 0.5);
        let wide = sdf_coverage(0.4, 4.0);
        assert!(tight < wide, "tight={tight} wide={wide}");
        assert!(tight < 0.5 && wide > 0.0);
    }

    #[test]
    fn stroke_coverage_peaks_on_contour_and_fades_both_sides() {
        assert_eq!(stroke_coverage(0.0, 1.0, 1.5), 1.0);
        assert!((stroke_coverage(1.5, 1.0, 1.5) - 0.5).abs() < 1e-6);
        assert!((stroke_coverage(-1.5, 1.0, 1.5) - 0.5).abs() < 1e-6);
        assert_eq!(stroke_coverage(3.0, 1.0, 1.5), 0.0);
        assert_eq!(stroke_coverage(-3.0, 1.0, 1.5), 0.0);
    }

    // ── Spec: sampling ───────────────────────────────────────────────────

    #[test]
    fn fwidth_of_a_true_sdf_in_pixel_space_is_about_one_to_sqrt2() {
        // Manhattan fwidth of a unit-gradient field: |Δx| + |Δy|. Along the
        // x-axis of a circle the x tap contributes ~1 and the y tap a small
        // curvature term (~0.05 at r=10); diagonally both taps contribute.
        let shape = |p: Vec2| sdf::circle_2d(p, Vec2::ZERO, 10.0);
        let axis = sample_coverage(shape, Vec2::new(10.0, 0.0));
        assert!(
            (axis.fwidth - 1.05).abs() < 0.01,
            "axis fwidth={}",
            axis.fwidth
        );
        let diag = sample_coverage(shape, Vec2::new(7.071, 7.071));
        // Forward differences at 45°: each tap moves ~0.73 along the
        // gradient, so the Manhattan sum lands near √2 (measured ≈1.46).
        assert!(
            (1.35..1.55).contains(&diag.fwidth) && diag.fwidth > axis.fwidth,
            "diag fwidth={}",
            diag.fwidth
        );
    }

    #[test]
    fn sample_coverage_inside_outside_and_on_edge() {
        let shape = |p: Vec2| sdf::circle_2d(p, Vec2::new(10.0, 10.0), 5.0);
        assert_eq!(sample_coverage(shape, Vec2::new(10.0, 10.0)).coverage, 1.0);
        assert_eq!(sample_coverage(shape, Vec2::new(40.0, 10.0)).coverage, 0.0);
        let edge = sample_coverage(shape, Vec2::new(15.0, 10.0));
        assert!((edge.coverage - 0.5).abs() < 1e-6, "edge={edge:?}");
    }

    #[test]
    fn scaled_sampling_keeps_the_edge_one_pixel_wide() {
        // Regression guard for the whole point of the module: zooming the
        // UI must not change *screen-space* edge softness. A point half a
        // pixel outside the edge has the same coverage at both scales.
        let shape = |p: Vec2| sdf::circle_2d(p, Vec2::ZERO, 1.0);
        let at_unit = sample_coverage_scaled(shape, Vec2::new(1.25, 0.0), Vec2::splat(0.5));
        let zoomed = sample_coverage_scaled(shape, Vec2::new(1.25, 0.0), Vec2::splat(0.05));
        // 0.25 world units out = half a pixel at step 0.5 → coverage ~0.11
        // (smoothstep, not linear); at step 0.05 it is 5 pixels out → 0.
        assert!(at_unit.coverage > 0.0 && at_unit.coverage < 0.5);
        assert_eq!(zoomed.coverage, 0.0);
        assert!(at_unit.fwidth > zoomed.fwidth);
    }

    // ── Rendering: happy path, edges, clip ───────────────────────────────

    fn panel() -> impl Fn(Vec2) -> f32 {
        |p: Vec2| sdf::rounded_rect_2d(p, Vec2::new(20.0, 12.0), Vec2::new(14.0, 8.0), 4.0)
    }

    #[test]
    fn fill_paints_interior_leaves_exterior_and_antialiases_the_edge() {
        let mut fb = Framebuffer::new(40, 24).unwrap();
        fill_sdf_shape(&mut fb, None, Color::WHITE, panel());
        assert_eq!(fb.get_pixel(20, 12), Some(Color::WHITE.to_argb_u32()));
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF00_0000));
        // Some pixel near the boundary must be a partial blend — that is
        // the anti-aliasing this module exists to produce. A hard
        // rasteriser would produce only pure black / pure white.
        let partial = fb
            .as_slice()
            .iter()
            .filter(|&&px| px != 0xFF00_0000 && px != Color::WHITE.to_argb_u32())
            .count();
        assert!(partial > 0, "no partially-covered edge pixels found");
    }

    #[test]
    fn fill_edge_is_smoother_than_a_hard_threshold() {
        // Along a horizontal scanline crossing the panel edge, coverage
        // (recovered from the red channel) must take an intermediate value
        // and be monotonic — not jump 0 → 255 in one pixel.
        let mut fb = Framebuffer::new(40, 24).unwrap();
        fill_sdf_shape(&mut fb, None, Color::WHITE, panel());
        let row: Vec<u32> = (0..40)
            .map(|x| (fb.get_pixel(x, 12).unwrap() >> 16) & 0xFF)
            .collect();
        let transitions = row.windows(2).filter(|w| w[0] != w[1]).count();
        assert!(transitions >= 2, "expected a multi-pixel ramp, row={row:?}");
    }

    #[test]
    fn fill_respects_the_clip_rectangle() {
        let mut fb = Framebuffer::new(40, 24).unwrap();
        let clip = IRect::from_pos_size(IVec2::new(0, 0), 20, 24);
        fill_sdf_shape(&mut fb, Some(clip), Color::WHITE, panel());
        assert_eq!(fb.get_pixel(10, 12), Some(Color::WHITE.to_argb_u32()));
        // Same shape, same row, but outside the clip: untouched.
        assert_eq!(fb.get_pixel(30, 12), Some(0xFF00_0000));
    }

    #[test]
    fn fill_with_empty_clip_paints_nothing() {
        let mut fb = Framebuffer::new(8, 8).unwrap();
        let clip = IRect::from_pos_size(IVec2::new(100, 100), 4, 4);
        fill_sdf_shape(&mut fb, Some(clip), Color::WHITE, panel());
        assert!(fb.as_slice().iter().all(|&px| px == 0xFF00_0000));
    }

    #[test]
    fn fill_blends_over_the_existing_background() {
        let mut fb = Framebuffer::new(40, 24).unwrap();
        fb.clear(Color::BLUE.to_argb_u32());
        fill_sdf_shape(&mut fb, None, Color::RED, panel());
        assert_eq!(fb.get_pixel(20, 12), Some(Color::RED.to_argb_u32()));
        assert_eq!(fb.get_pixel(0, 0), Some(Color::BLUE.to_argb_u32()));
        // An edge pixel is a genuine mix of red and blue.
        let mixed = fb.as_slice().iter().any(|&px| {
            let r = (px >> 16) & 0xFF;
            let b = px & 0xFF;
            r > 0 && r < 255 && b > 0 && b < 255
        });
        assert!(mixed, "no red/blue blended edge pixel found");
    }

    #[test]
    fn stroke_paints_a_ring_not_a_disk() {
        let mut fb = Framebuffer::new(32, 32).unwrap();
        stroke_sdf_shape(&mut fb, None, Color::WHITE, 1.0, |p| {
            sdf::circle_2d(p, Vec2::new(16.0, 16.0), 10.0)
        });
        assert_ne!(fb.get_pixel(26, 16), Some(0xFF00_0000));
        assert_eq!(fb.get_pixel(16, 16), Some(0xFF00_0000));
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF00_0000));
    }

    #[test]
    fn composed_ui_panel_with_pill_button() {
        // The intended use, end to end: a rounded panel (fill) plus a
        // capsule "button" (fill, different colour) composed from the
        // existing primitives — no new shape code needed.
        let mut fb = Framebuffer::new(64, 32).unwrap();
        fill_sdf_shape(&mut fb, None, Color::grey(0.25), |p| {
            sdf::rounded_rect_2d(p, Vec2::new(32.0, 16.0), Vec2::new(30.0, 14.0), 6.0)
        });
        fill_sdf_shape(&mut fb, None, Color::GREEN, |p| {
            sdf::capsule_2d(p, Vec2::new(24.0, 16.0), Vec2::new(40.0, 16.0), 6.0)
        });
        assert_eq!(fb.get_pixel(32, 16), Some(Color::GREEN.to_argb_u32()));
        let panel_px = fb.get_pixel(6, 16).unwrap();
        assert_eq!(panel_px, Color::grey(0.25).to_argb_u32());
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF00_0000));
    }

    #[test]
    fn proptest_coverage_stays_in_unit_range() {
        // Deterministic sweep standing in for a property test over the
        // (d, fwidth) plane, including degenerate widths.
        for d_bits in -40..=40 {
            for w_bits in -4..=40 {
                let c = sdf_coverage(d_bits as f32 * 0.25, w_bits as f32 * 0.25);
                assert!((0.0..=1.0).contains(&c), "d={d_bits} w={w_bits} c={c}");
                assert!(c.is_finite());
            }
        }
    }
}
