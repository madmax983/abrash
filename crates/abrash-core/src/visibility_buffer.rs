#![allow(
    clippy::float_cmp,
    clippy::imprecise_flops,
    clippy::too_long_first_doc_paragraph
)]
//! CPU visibility buffer: rasterise *identity*, shade later.
//!
//! A forward renderer shades every fragment a triangle covers — including
//! the fragments a nearer triangle is about to paint over. A visibility
//! buffer (the deferred geometry pass behind id Tech 8's "visbuffer"
//! direction and most modern deferred pipelines) splits the work in two:
//!
//! 1. **Rasterise** ([`VisibilityBuffer::rasterize`]): triangles write only
//!    *which triangle* covers each pixel, the pixel's barycentric
//!    coordinates inside it, and the winning depth. No shading, no
//!    material code, no texture work — overdraw costs a few integer/float
//!    writes, not a full shader run.
//! 2. **Resolve** ([`VisibilityBuffer::resolve`]): each covered pixel is
//!    shaded exactly once, reconstructing its interpolated attributes
//!    (normal, UV, vertex colour) from the stored barycentrics and the
//!    triangle's vertex data. Materials can branch, textures can filter —
//!    the cost is paid per *visible* pixel, never per covered fragment.
//!
//! Because the resolve pass works from barycentrics, it also gets the
//! classic visbuffer extras for free: perspective-correct interpolation
//! from per-vertex `inv_w`, and screen-space UV derivatives computed from
//! neighbouring pixels that belong to the same triangle
//! ([`VisibilityBuffer::uv_derivatives`]) — the inputs texture LOD
//! selection needs, with no analytic derivatives anywhere.
//!
//! # Conventions
//!
//! * Geometry arrives **already projected**: [`VisVertex::position`] is
//!   screen space in pixels (x right, y down, pixel centres at half
//!   integers) and `position.z` is screen-space depth. **Smaller depth
//!   wins**, matching [`crate::zbuffer::ZBuffer`]; clear depth is
//!   `f32::INFINITY`.
//! * There is **no backface culling**: either winding rasterises (the
//!   rasteriser normalises winding internally). Culling is a separate
//!   concern that belongs before this pass.
//! * Shared edges are watertight: the top-left fill rule assigns every
//!   on-edge pixel centre to exactly one of the two triangles sharing the
//!   edge, so a triangulated quad covers its rectangle with no gaps and
//!   no double-writes.
//! * Depth ties keep the **earlier** triangle in the input slice, so the
//!   result is deterministic for a given input order.
//!
//! # Invariants (the spec these functions are tested against)
//!
//! 1. A pixel is covered iff its stored triangle id is not
//!    [`NO_TRIANGLE`]; covered pixels have barycentrics in `[0, 1]`
//!    summing to 1 (up to float rounding) and a depth between the
//!    triangle's minimum and maximum vertex depths.
//! 2. Two triangles sharing an edge never both cover the same pixel, and
//!    their union has no holes along the edge.
//! 3. The nearest triangle wins a pixel regardless of submission order;
//!    on an exact depth tie the earlier submission wins.
//! 4. At a triangle's centroid with all `inv_w == 1`, reconstruction
//!    returns the plain average of the three vertices' attributes.
//! 5. Resolve shades every covered pixel exactly once and writes the
//!    background colour to every uncovered pixel of the output.

use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::math::{Vec2, Vec3};

/// Triangle id stored in uncovered pixels.
pub const NO_TRIANGLE: u32 = u32::MAX;

/// Material used when a triangle's material index is out of range:
/// unmistakable magenta, so a bad index is visible instead of silent.
pub const FALLBACK_MATERIAL: VisMaterial = VisMaterial {
    base_color: Color::MAGENTA,
    emissive: Color::BLACK,
};

/// One vertex of a visibility-buffer triangle, already projected to
/// screen space.
///
/// `inv_w` is `1 / w` from the projection (1.0 for orthographic or
/// pre-divided geometry). The rasteriser interpolates in screen space;
/// the resolve pass uses `inv_w` to recover perspective-correct
/// barycentrics before interpolating attributes, exactly like a GPU
/// interpolator. It must be positive for the correction to be valid;
/// non-positive sums fall back to the raw screen-space barycentrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisVertex {
    /// Screen-space position: x/y in pixels, z = depth (smaller wins).
    pub position: Vec3,
    /// `1 / w` from projection; 1.0 when there is no perspective divide.
    pub inv_w: f32,
    /// Vertex normal (need not be unit length; resolve normalises).
    pub normal: Vec3,
    /// Texture coordinates.
    pub uv: Vec2,
    /// Vertex colour, multiplied with the material colour at resolve.
    pub color: Color,
}

impl VisVertex {
    /// A vertex at screen position `(x, y, depth)` with `inv_w = 1`,
    /// a +Z normal, zero UVs and a white vertex colour — the plain
    /// starting point most fixtures want.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::visibility_buffer::VisVertex;
    ///
    /// let v = VisVertex::screen(10.0, 20.0, 0.5);
    /// assert_eq!(v.inv_w, 1.0);
    /// assert_eq!(v.position.z, 0.5);
    /// ```
    #[must_use]
    pub const fn screen(x: f32, y: f32, depth: f32) -> Self {
        Self {
            position: Vec3::new(x, y, depth),
            inv_w: 1.0,
            normal: Vec3::new(0.0, 0.0, 1.0),
            uv: Vec2::ZERO,
            color: Color::WHITE,
        }
    }

    /// Builder: set the perspective `1 / w`.
    #[must_use]
    pub const fn with_inv_w(mut self, inv_w: f32) -> Self {
        self.inv_w = inv_w;
        self
    }

    /// Builder: set the vertex normal.
    #[must_use]
    pub const fn with_normal(mut self, normal: Vec3) -> Self {
        self.normal = normal;
        self
    }

    /// Builder: set the texture coordinates.
    #[must_use]
    pub const fn with_uv(mut self, uv: Vec2) -> Self {
        self.uv = uv;
        self
    }

    /// Builder: set the vertex colour.
    #[must_use]
    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

/// A screen-space triangle plus the material it shades with at resolve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisTriangle {
    /// The three corners, in either winding (winding is normalised).
    pub verts: [VisVertex; 3],
    /// Index into the material slice passed to
    /// [`VisibilityBuffer::resolve`]; out-of-range indices shade with
    /// [`FALLBACK_MATERIAL`].
    pub material: u32,
}

impl VisTriangle {
    /// A triangle from three vertices and a material index.
    #[must_use]
    pub const fn new(verts: [VisVertex; 3], material: u32) -> Self {
        Self { verts, material }
    }
}

/// The shading inputs for one material: a base colour modulated by the
/// interpolated vertex colour and a Lambert term, plus an emissive term
/// that ignores lighting entirely.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisMaterial {
    /// Base (albedo) colour in linear space.
    pub base_color: Color,
    /// Emissive colour in linear space, added unlit.
    pub emissive: Color,
}

impl Default for VisMaterial {
    fn default() -> Self {
        Self {
            base_color: Color::WHITE,
            emissive: Color::BLACK,
        }
    }
}

impl VisMaterial {
    /// A plain unlit-by-emission material with the given base colour.
    #[must_use]
    pub const fn plain(base_color: Color) -> Self {
        Self {
            base_color,
            emissive: Color::BLACK,
        }
    }
}

/// What one pixel of the visibility buffer stores: the winning triangle,
/// two barycentric coordinates (the third is `1 - u - v`), and the
/// winning depth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisPixel {
    /// Winning triangle index, or [`NO_TRIANGLE`] when uncovered.
    pub triangle: u32,
    /// Barycentric weights of vertices 1 and 2 (`bary[0]` ↔ vertex 1).
    /// Vertex 0's weight is `1.0 - bary[0] - bary[1]`.
    pub bary: [f32; 2],
    /// Winning screen-space depth (smaller = nearer).
    pub depth: f32,
}

impl VisPixel {
    /// The cleared state: no triangle, infinite depth.
    pub const EMPTY: Self = Self {
        triangle: NO_TRIANGLE,
        bary: [0.0, 0.0],
        depth: f32::INFINITY,
    };

    /// All three barycentric weights, `(w0, w1, w2)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::visibility_buffer::VisPixel;
    ///
    /// let p = VisPixel { triangle: 3, bary: [0.25, 0.25], depth: 1.0 };
    /// assert_eq!(p.bary3(), (0.5, 0.25, 0.25));
    /// ```
    #[must_use]
    pub fn bary3(self) -> (f32, f32, f32) {
        (
            1.0 - self.bary[0] - self.bary[1],
            self.bary[0],
            self.bary[1],
        )
    }

    /// Whether any triangle covers this pixel.
    #[must_use]
    pub const fn is_covered(self) -> bool {
        self.triangle != NO_TRIANGLE
    }
}

/// Counters from one [`VisibilityBuffer::rasterize`] call, for the
/// measure-first crowd: a forward renderer would shade every
/// `fragments_tested` fragment; the visibility buffer shades only the
/// `covered_pixels` survivors, so `fragments_tested - covered_pixels`
/// is the shading work the split absorbed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VisStats {
    /// Triangles submitted.
    pub triangles_input: usize,
    /// Triangles skipped as degenerate (zero screen-space area).
    pub triangles_degenerate: usize,
    /// Pixel centres passing the fill rule (before the depth test).
    pub fragments_tested: usize,
    /// Fragments that won the depth test and were written.
    pub fragments_written: usize,
    /// Pixels covered in the buffer after the pass.
    pub covered_pixels: usize,
}

/// Lighting and background settings for [`VisibilityBuffer::resolve`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolveParams {
    /// Direction *towards* the light (normalised internally).
    pub light_dir: Vec3,
    /// Fraction of the base colour that is always present, in `[0, 1]`.
    pub ambient: f32,
    /// Packed `0xAARRGGBB` colour written to uncovered pixels.
    pub background: u32,
}

impl Default for ResolveParams {
    fn default() -> Self {
        Self {
            light_dir: Vec3::new(0.35, -0.45, 0.82),
            ambient: 0.2,
            background: 0xFF00_0000,
        }
    }
}

/// A pixel's attributes reconstructed at resolve time: everything a
/// material shader needs, rebuilt from the stored barycentrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfacePoint {
    /// Index of the triangle covering the pixel.
    pub triangle: u32,
    /// Material index of that triangle.
    pub material: u32,
    /// Perspective-correct barycentric weights `(w0, w1, w2)`.
    pub bary: [f32; 3],
    /// Interpolated normal, normalised (zero if the inputs cancel out).
    pub normal: Vec3,
    /// Interpolated texture coordinates.
    pub uv: Vec2,
    /// Interpolated vertex colour.
    pub color: Color,
    /// The pixel's winning depth.
    pub depth: f32,
}

/// A width × height grid of [`VisPixel`]s: the rasterise target and the
/// resolve source.
///
/// # Examples
///
/// ```
/// use abrash_core::math::Vec3;
/// use abrash_core::visibility_buffer::{VisTriangle, VisVertex, VisibilityBuffer};
///
/// let tri = VisTriangle::new(
///     [
///         VisVertex::screen(2.0, 2.0, 0.5),
///         VisVertex::screen(14.0, 2.0, 0.5),
///         VisVertex::screen(2.0, 14.0, 0.5),
///     ],
///     0,
/// );
/// let mut vis = VisibilityBuffer::new(16, 16).unwrap();
/// let stats = vis.rasterize(&[tri]);
/// assert!(stats.covered_pixels > 0);
///
/// let centre = vis.reconstruct(&[tri], 4, 4).unwrap();
/// assert_eq!(centre.triangle, 0);
/// let sum: f32 = centre.bary.iter().sum();
/// assert!((sum - 1.0).abs() < 1e-5);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct VisibilityBuffer {
    width: u32,
    height: u32,
    pixels: Vec<VisPixel>,
}

impl VisibilityBuffer {
    /// Creates an empty (fully uncovered) visibility buffer.
    ///
    /// # Errors
    ///
    /// Returns an error for zero dimensions, dimensions above
    /// `i32::MAX`, or a pixel count that overflows `u32` — the same
    /// contract as [`Framebuffer::new`], plus the zero check.
    pub fn new(width: u32, height: u32) -> Result<Self, &'static str> {
        if width == 0 || height == 0 {
            return Err("Buffer dimensions must be non-zero");
        }
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
            pixels: vec![VisPixel::EMPTY; size],
        })
    }

    /// Buffer width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Buffer height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Resets every pixel to [`VisPixel::EMPTY`].
    pub fn clear(&mut self) {
        self.pixels.fill(VisPixel::EMPTY);
    }

    /// The stored record at `(x, y)`, or `None` out of bounds.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<VisPixel> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.pixels[(y * self.width + x) as usize])
    }

    /// All stored records, row-major.
    #[must_use]
    pub fn as_slice(&self) -> &[VisPixel] {
        &self.pixels
    }

    /// How many pixels any triangle covers.
    #[must_use]
    pub fn covered_count(&self) -> usize {
        self.pixels.iter().filter(|p| p.is_covered()).count()
    }

    /// Rasterises `triangles` into the buffer, clearing it first.
    ///
    /// Each triangle is normalised to a consistent winding, clipped to
    /// the buffer by its bounding box, and tested per pixel centre with
    /// edge functions and the top-left fill rule. The nearest depth
    /// wins; ties keep the earlier triangle.
    pub fn rasterize(&mut self, triangles: &[VisTriangle]) -> VisStats {
        self.clear();
        let mut stats = VisStats {
            triangles_input: triangles.len(),
            ..VisStats::default()
        };
        for (index, tri) in triangles.iter().enumerate() {
            rasterize_triangle(self, index as u32, tri, &mut stats);
        }
        stats.covered_pixels = self.covered_count();
        stats
    }

    /// Reconstructs the interpolated attributes of pixel `(x, y)` from
    /// its stored barycentrics and the triangle list it was rasterised
    /// from, applying perspective correction with the vertices'
    /// `inv_w`. Returns `None` for uncovered or out-of-bounds pixels
    /// (and for a stored id outside `triangles`, which can only happen
    /// if the slices disagree).
    #[must_use]
    pub fn reconstruct(&self, triangles: &[VisTriangle], x: u32, y: u32) -> Option<SurfacePoint> {
        let stored = self.pixel(x, y)?;
        if !stored.is_covered() {
            return None;
        }
        let tri = triangles.get(stored.triangle as usize)?;
        let (bary, normal, uv, color) = interpolate(tri, stored);
        Some(SurfacePoint {
            triangle: stored.triangle,
            material: tri.material,
            bary,
            normal,
            uv,
            color,
            depth: stored.depth,
        })
    }

    /// Screen-space UV derivatives `(dUV/dx, dUV/dy)` at pixel `(x, y)`,
    /// from the reconstructed UVs of the +x / +y neighbours — defined
    /// only when both neighbours are covered by the *same* triangle,
    /// which is exactly the condition under which a GPU quad derivative
    /// is meaningful. Returns `None` at silhouette and material edges.
    #[must_use]
    pub fn uv_derivatives(
        &self,
        triangles: &[VisTriangle],
        x: u32,
        y: u32,
    ) -> Option<(Vec2, Vec2)> {
        let here = self.reconstruct(triangles, x, y)?;
        let right = self.reconstruct(triangles, x + 1, y)?;
        let down = self.reconstruct(triangles, x, y + 1)?;
        if right.triangle != here.triangle || down.triangle != here.triangle {
            return None;
        }
        Some((right.uv - here.uv, down.uv - here.uv))
    }

    /// Shades every pixel once into `out`: covered pixels get their
    /// material's Lambert shading, uncovered pixels get
    /// `params.background`. Only the overlapping region is written when
    /// `out` and the buffer differ in size. Returns the number of
    /// covered (actually shaded) pixels.
    pub fn resolve(
        &self,
        triangles: &[VisTriangle],
        materials: &[VisMaterial],
        params: &ResolveParams,
        out: &mut Framebuffer,
    ) -> usize {
        let light = params.light_dir.normalize_or_zero();
        let ambient = params.ambient.clamp(0.0, 1.0);
        let w = self.width.min(out.width());
        let h = self.height.min(out.height());
        let mut shaded = 0_usize;
        for y in 0..h {
            for x in 0..w {
                let Some(point) = self.reconstruct(triangles, x, y) else {
                    out.set_pixel(x as i32, y as i32, params.background);
                    continue;
                };
                let material = materials
                    .get(point.material as usize)
                    .copied()
                    .unwrap_or(FALLBACK_MATERIAL);
                let ndotl = point.normal.dot(light).max(0.0);
                let diffuse = ambient + (1.0 - ambient) * ndotl;
                let base = material.base_color;
                let v = point.color;
                let lit = Color::new(
                    material.emissive.r + base.r * v.r * diffuse,
                    material.emissive.g + base.g * v.g * diffuse,
                    material.emissive.b + base.b * v.b * diffuse,
                    base.a * v.a,
                )
                .saturate();
                out.set_pixel(x as i32, y as i32, lit.to_argb_u32());
                shaded += 1;
            }
        }
        shaded
    }
}

/// Twice the signed area of `(a, b, p)`: positive when `p` is on the
/// interior side of edge `a → b` for a positively-wound triangle.
fn edge_function(a: Vec3, b: Vec3, px: f32, py: f32) -> f32 {
    (b.x - a.x) * (py - a.y) - (b.y - a.y) * (px - a.x)
}

/// Top-left rule for edge `a → b` (y-down screen space): of the two
/// directions an edge can be traversed, exactly one is "top-left", so a
/// pixel centre exactly on a shared edge belongs to exactly one of the
/// two triangles that share it.
const fn is_top_left(a: Vec3, b: Vec3) -> bool {
    (a.y == b.y && a.x > b.x) || a.y > b.y
}

/// Whether edge weight `w` along edge `a → b` counts as inside.
fn inside(w: f32, a: Vec3, b: Vec3) -> bool {
    w > 0.0 || (w == 0.0 && is_top_left(a, b))
}

fn rasterize_triangle(
    vis: &mut VisibilityBuffer,
    index: u32,
    tri: &VisTriangle,
    stats: &mut VisStats,
) {
    let mut v = tri.verts;
    let area2 = edge_function(
        v[0].position,
        v[1].position,
        v[2].position.x,
        v[2].position.y,
    );
    if area2 == 0.0 || !area2.is_finite() {
        stats.triangles_degenerate += 1;
        return;
    }
    let swapped = area2 < 0.0;
    if swapped {
        v.swap(1, 2);
    }
    let area2 = area2.abs();
    let (p0, p1, p2) = (v[0].position, v[1].position, v[2].position);

    // Bounding box of pixel centres that could be covered, clamped to
    // the buffer. Pixel centres sit at half-integers, so the candidate
    // range is ceil(min - 0.5) ..= floor(max - 0.5) per axis.
    let min_x = p0.x.min(p1.x).min(p2.x);
    let max_x = p0.x.max(p1.x).max(p2.x);
    let min_y = p0.y.min(p1.y).min(p2.y);
    let max_y = p0.y.max(p1.y).max(p2.y);
    let x0 = ((min_x - 0.5).ceil() as i32).max(0);
    let y0 = ((min_y - 0.5).ceil() as i32).max(0);
    let x1 = ((max_x - 0.5).floor() as i32).min(vis.width as i32 - 1);
    let y1 = ((max_y - 0.5).floor() as i32).min(vis.height as i32 - 1);
    if x0 > x1 || y0 > y1 {
        return;
    }

    for y in y0..=y1 {
        let py = y as f32 + 0.5;
        for x in x0..=x1 {
            let px = x as f32 + 0.5;
            let w0 = edge_function(p1, p2, px, py);
            let w1 = edge_function(p2, p0, px, py);
            let w2 = edge_function(p0, p1, px, py);
            if !(inside(w0, p1, p2) && inside(w1, p2, p0) && inside(w2, p0, p1)) {
                continue;
            }
            stats.fragments_tested += 1;
            let l0 = w0 / area2;
            let l1 = w1 / area2;
            let l2 = w2 / area2;
            let depth = l0 * p0.z + l1 * p1.z + l2 * p2.z;
            let idx = (y as u32 * vis.width + x as u32) as usize;
            // Strict <: an exact tie keeps the earlier triangle.
            if depth < vis.pixels[idx].depth {
                // Stored barycentrics are indexed by the *original*
                // vertex order; undo the winding swap so resolve maps
                // them onto the right vertices.
                let bary = if swapped { [l2, l1] } else { [l1, l2] };
                vis.pixels[idx] = VisPixel {
                    triangle: index,
                    bary,
                    depth,
                };
                stats.fragments_written += 1;
            }
        }
    }
}

/// Perspective-correct interpolation of one stored pixel: returns the
/// corrected barycentrics and the interpolated normal, UV and colour.
fn interpolate(tri: &VisTriangle, stored: VisPixel) -> ([f32; 3], Vec3, Vec2, Color) {
    let (l0, l1, l2) = stored.bary3();
    let raw = [l0, l1, l2];
    let weighted = [
        l0 * tri.verts[0].inv_w,
        l1 * tri.verts[1].inv_w,
        l2 * tri.verts[2].inv_w,
    ];
    let sum = weighted[0] + weighted[1] + weighted[2];
    let bary = if sum > 0.0 && sum.is_finite() {
        [weighted[0] / sum, weighted[1] / sum, weighted[2] / sum]
    } else {
        raw
    };
    let blend3 = |f: &dyn Fn(&VisVertex) -> Vec3| -> Vec3 {
        f(&tri.verts[0]) * bary[0] + f(&tri.verts[1]) * bary[1] + f(&tri.verts[2]) * bary[2]
    };
    let normal = blend3(&|v| v.normal).normalize_or_zero();
    let uv = Vec2::new(
        tri.verts[0].uv.x * bary[0] + tri.verts[1].uv.x * bary[1] + tri.verts[2].uv.x * bary[2],
        tri.verts[0].uv.y * bary[0] + tri.verts[1].uv.y * bary[1] + tri.verts[2].uv.y * bary[2],
    );
    let color = Color::new(
        tri.verts[0].color.r * bary[0]
            + tri.verts[1].color.r * bary[1]
            + tri.verts[2].color.r * bary[2],
        tri.verts[0].color.g * bary[0]
            + tri.verts[1].color.g * bary[1]
            + tri.verts[2].color.g * bary[2],
        tri.verts[0].color.b * bary[0]
            + tri.verts[1].color.b * bary[1]
            + tri.verts[2].color.b * bary[2],
        tri.verts[0].color.a * bary[0]
            + tri.verts[1].color.a * bary[1]
            + tri.verts[2].color.a * bary[2],
    );
    (bary, normal, uv, color)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn tri_screen(a: (f32, f32), b: (f32, f32), c: (f32, f32), depth: f32) -> VisTriangle {
        VisTriangle::new(
            [
                VisVertex::screen(a.0, a.1, depth),
                VisVertex::screen(b.0, b.1, depth),
                VisVertex::screen(c.0, c.1, depth),
            ],
            0,
        )
    }

    fn covered_set(vis: &VisibilityBuffer) -> Vec<(u32, u32, u32)> {
        let mut out = Vec::new();
        for y in 0..vis.height() {
            for x in 0..vis.width() {
                let p = vis.pixel(x, y).unwrap();
                if p.is_covered() {
                    out.push((x, y, p.triangle));
                }
            }
        }
        out
    }

    #[test]
    fn new_buffer_is_empty_and_rejects_zero_dimensions() {
        let vis = VisibilityBuffer::new(8, 8).unwrap();
        assert_eq!(vis.covered_count(), 0);
        assert_eq!(vis.pixel(0, 0), Some(VisPixel::EMPTY));
        assert_eq!(vis.pixel(8, 0), None);
        assert!(VisibilityBuffer::new(0, 8).is_err());
        assert!(VisibilityBuffer::new(8, 0).is_err());
    }

    #[test]
    fn single_triangle_covers_interior_with_valid_barycentrics() {
        let tri = tri_screen((2.0, 2.0), (14.0, 2.0), (2.0, 14.0), 0.5);
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        let stats = vis.rasterize(&[tri]);
        assert!(stats.covered_pixels > 40);
        assert_eq!(stats.covered_pixels, vis.covered_count());
        assert_eq!(stats.fragments_written, stats.covered_pixels);
        for (_, _, id) in covered_set(&vis) {
            assert_eq!(id, 0);
        }
        for y in 0..16 {
            for x in 0..16 {
                let p = vis.pixel(x, y).unwrap();
                if p.is_covered() {
                    let (b0, b1, b2) = p.bary3();
                    assert!((0.0..=1.0).contains(&b0), "b0={b0}");
                    assert!((0.0..=1.0).contains(&b1), "b1={b1}");
                    assert!((0.0..=1.0).contains(&b2), "b2={b2}");
                    assert!((b0 + b1 + b2 - 1.0).abs() < 1e-5);
                    assert!((p.depth - 0.5).abs() < EPS);
                }
            }
        }
        // Well outside the hypotenuse: uncovered.
        assert!(!vis.pixel(13, 13).unwrap().is_covered());
    }

    #[test]
    fn vertex_pixel_has_dominant_barycentric() {
        // Pixel centre (2.5, 2.5) is nearest vertex 0 at (2, 2).
        let tri = tri_screen((2.0, 2.0), (14.0, 2.0), (2.0, 14.0), 0.5);
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        vis.rasterize(&[tri]);
        let (b0, b1, b2) = vis.pixel(2, 2).unwrap().bary3();
        assert!(b0 > b1 && b0 > b2, "bary=({b0},{b1},{b2})");
        assert!(b0 > 0.85);
    }

    #[test]
    fn shared_edge_is_watertight_no_gaps_no_overlap() {
        // Two triangles tiling the square [2,14] x [2,14] exactly.
        let t0 = tri_screen((2.0, 2.0), (14.0, 2.0), (14.0, 14.0), 0.5);
        let mut t1 = tri_screen((2.0, 2.0), (14.0, 14.0), (2.0, 14.0), 0.5);
        t1.material = 1;
        let tris = [t0, t1];
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        let stats = vis.rasterize(&tris);
        // The square holds exactly 12x12 pixel centres.
        assert_eq!(stats.covered_pixels, 144);
        // No pixel was written twice: writes == tested == covered.
        assert_eq!(stats.fragments_tested, 144);
        assert_eq!(stats.fragments_written, 144);
        for y in 2..14 {
            for x in 2..14 {
                assert!(vis.pixel(x, y).unwrap().is_covered(), "hole at ({x},{y})");
            }
        }
    }

    #[test]
    fn reversed_winding_covers_the_same_pixels_and_attributes() {
        let v0 = VisVertex::screen(2.0, 2.0, 0.5).with_uv(Vec2::new(0.0, 0.0));
        let v1 = VisVertex::screen(14.0, 2.0, 0.5).with_uv(Vec2::new(1.0, 0.0));
        let v2 = VisVertex::screen(2.0, 14.0, 0.5).with_uv(Vec2::new(0.0, 1.0));
        let fwd = VisTriangle::new([v0, v1, v2], 0);
        // Same vertices, reversed winding, vertex order preserved as a
        // set: index 0 is still the (2,2) corner conceptually — build
        // the reversed triangle as [v0, v2, v1] *positions* but note the
        // module keys barycentrics to each triangle's own vertex order,
        // so compare reconstructed UVs (a function of position), which
        // must agree pixel for pixel.
        let rev = VisTriangle::new([v0, v2, v1], 0);
        let mut a = VisibilityBuffer::new(16, 16).unwrap();
        let mut b = VisibilityBuffer::new(16, 16).unwrap();
        a.rasterize(&[fwd]);
        b.rasterize(&[rev]);
        let set_a: Vec<(u32, u32)> = covered_set(&a)
            .into_iter()
            .map(|(x, y, _)| (x, y))
            .collect();
        let set_b: Vec<(u32, u32)> = covered_set(&b)
            .into_iter()
            .map(|(x, y, _)| (x, y))
            .collect();
        assert_eq!(set_a, set_b);
        for (x, y) in set_a {
            let pa = a.reconstruct(&[fwd], x, y).unwrap();
            let pb = b.reconstruct(&[rev], x, y).unwrap();
            assert!(
                (pa.uv.x - pb.uv.x).abs() < 1e-5 && (pa.uv.y - pb.uv.y).abs() < 1e-5,
                "uv mismatch at ({x},{y}): {:?} vs {:?}",
                pa.uv,
                pb.uv
            );
        }
    }

    #[test]
    fn degenerate_triangles_cover_nothing_and_are_counted() {
        let collinear = tri_screen((2.0, 2.0), (8.0, 2.0), (14.0, 2.0), 0.5);
        let point = tri_screen((5.0, 5.0), (5.0, 5.0), (5.0, 5.0), 0.5);
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        let stats = vis.rasterize(&[collinear, point]);
        assert_eq!(stats.triangles_degenerate, 2);
        assert_eq!(stats.covered_pixels, 0);
    }

    #[test]
    fn nearer_triangle_wins_regardless_of_submission_order() {
        let far = tri_screen((2.0, 2.0), (14.0, 2.0), (2.0, 14.0), 0.8);
        let mut near = tri_screen((2.0, 2.0), (14.0, 2.0), (2.0, 14.0), 0.3);
        near.material = 1;
        for tris in [[far, near], [near, far]] {
            let mut vis = VisibilityBuffer::new(16, 16).unwrap();
            vis.rasterize(&tris);
            let winner_id = u32::from(tris[0].material != 1);
            let p = vis.pixel(4, 4).unwrap();
            assert_eq!(p.triangle, winner_id);
            assert!((p.depth - 0.3).abs() < EPS);
        }
    }

    #[test]
    fn depth_tie_keeps_the_earlier_triangle() {
        let first = tri_screen((2.0, 2.0), (14.0, 2.0), (2.0, 14.0), 0.5);
        let second = tri_screen((2.0, 2.0), (14.0, 2.0), (2.0, 14.0), 0.5);
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        vis.rasterize(&[first, second]);
        assert_eq!(vis.pixel(4, 4).unwrap().triangle, 0);
    }

    #[test]
    fn offscreen_triangles_are_clipped_not_fatal() {
        let partial = tri_screen((-6.0, 2.0), (10.0, 2.0), (-6.0, 14.0), 0.5);
        let gone = tri_screen((-30.0, -30.0), (-20.0, -30.0), (-30.0, -20.0), 0.5);
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        let stats = vis.rasterize(&[partial, gone]);
        assert!(stats.covered_pixels > 0);
        assert!(vis.pixel(0, 4).unwrap().is_covered());
        assert_eq!(vis.pixel(0, 4).unwrap().triangle, 0);
    }

    #[test]
    fn reconstruct_at_centroid_averages_attributes() {
        let mk = |x: f32, y: f32, uv: Vec2, c: Color| {
            VisVertex::screen(x, y, 0.5).with_uv(uv).with_color(c)
        };
        let tri = VisTriangle::new(
            [
                mk(2.0, 2.0, Vec2::new(0.0, 0.0), Color::RED),
                mk(14.0, 2.0, Vec2::new(1.0, 0.0), Color::GREEN),
                mk(2.0, 14.0, Vec2::new(0.0, 1.0), Color::BLUE),
            ],
            7,
        );
        // Centroid of (2,2),(14,2),(2,14) is (6,6); pixel (5,5)'s
        // centre (5.5,5.5) is near it but not exact — use the stored
        // barycentrics to check interpolation against the analytic blend
        // at the same weights instead of assuming the centroid pixel.
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        vis.rasterize(&[tri]);
        let point = vis.reconstruct(&[tri], 5, 5).unwrap();
        assert_eq!(point.material, 7);
        let [b0, b1, b2] = point.bary;
        assert!((point.uv.x - b1).abs() < 1e-5);
        assert!((point.uv.y - b2).abs() < 1e-5);
        assert!((point.color.r - b0).abs() < 1e-5);
        assert!((point.color.g - b1).abs() < 1e-5);
        assert!((point.color.b - b2).abs() < 1e-5);
        // Default normals are all +Z, so the blend is +Z.
        assert!((point.normal.z - 1.0).abs() < 1e-5);
    }

    #[test]
    fn perspective_correction_shifts_weight_towards_smaller_inv_w() {
        // At the screen-space centroid the raw barycentrics are thirds.
        // inv_w = (1, 0.5, 0.5) must correct them to (0.5, 0.25, 0.25).
        let tri = VisTriangle::new(
            [
                VisVertex::screen(2.0, 2.0, 0.5).with_inv_w(1.0),
                VisVertex::screen(14.0, 2.0, 0.5).with_inv_w(0.5),
                VisVertex::screen(2.0, 14.0, 0.5).with_inv_w(0.5),
            ],
            0,
        );
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        vis.rasterize(&[tri]);
        // Find the covered pixel whose raw barycentrics are closest to
        // thirds and check its corrected weights against the formula.
        let mut best: Option<(f32, [f32; 3], [f32; 3])> = None;
        for y in 0..16 {
            for x in 0..16 {
                if let Some(p) = vis.reconstruct(&[tri], x, y) {
                    let (r0, r1, r2) = vis.pixel(x, y).unwrap().bary3();
                    let err =
                        (r0 - 1.0 / 3.0).abs() + (r1 - 1.0 / 3.0).abs() + (r2 - 1.0 / 3.0).abs();
                    if best.is_none_or(|(e, _, _)| err < e) {
                        best = Some((err, [r0, r1, r2], p.bary));
                    }
                }
            }
        }
        let (_, raw, corrected) = best.unwrap();
        // Exact expectation from the raw weights: w_i ∝ raw_i * inv_w_i.
        let w = [raw[0], raw[1] * 0.5, raw[2] * 0.5];
        let s = w[0] + w[1] + w[2];
        for i in 0..3 {
            assert!((corrected[i] - w[i] / s).abs() < 1e-5, "bary[{i}]");
        }
        // The corrected weight of vertex 0 must exceed its raw weight:
        // its inv_w is the largest, so perspective correction pulls
        // weight towards it.
        assert!(corrected[0] > raw[0], "corrected={corrected:?} raw={raw:?}");
        let sum: f32 = corrected.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
    }

    #[test]
    fn reconstruct_uncovered_pixel_is_none() {
        let tri = tri_screen((2.0, 2.0), (14.0, 2.0), (2.0, 14.0), 0.5);
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        vis.rasterize(&[tri]);
        assert!(vis.reconstruct(&[tri], 13, 13).is_none());
        assert!(vis.reconstruct(&[tri], 99, 99).is_none());
    }

    #[test]
    fn uv_derivatives_match_a_planar_uv_ramp() {
        // UV = position / 16 over the whole square: du/dx = dv/dy = 1/16.
        let mk =
            |x: f32, y: f32| VisVertex::screen(x, y, 0.5).with_uv(Vec2::new(x / 16.0, y / 16.0));
        let tri = VisTriangle::new([mk(0.0, 0.0), mk(16.0, 0.0), mk(0.0, 16.0)], 0);
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        vis.rasterize(&[tri]);
        let (dudx, dudy) = vis.uv_derivatives(&[tri], 3, 3).unwrap();
        assert!((dudx.x - 1.0 / 16.0).abs() < 1e-5, "dudx={dudx:?}");
        assert!(dudx.y.abs() < 1e-5);
        assert!(dudy.x.abs() < 1e-5);
        assert!((dudy.y - 1.0 / 16.0).abs() < 1e-5, "dudy={dudy:?}");
        // At the hypotenuse the +x neighbour belongs to no triangle.
        assert!(vis.uv_derivatives(&[tri], 12, 3).is_none());
    }

    #[test]
    fn resolve_shades_covered_and_paints_background_elsewhere() {
        let tri = tri_screen((2.0, 2.0), (14.0, 2.0), (2.0, 14.0), 0.5);
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        let stats = vis.rasterize(&[tri]);
        let materials = [VisMaterial::plain(Color::WHITE)];
        let params = ResolveParams {
            light_dir: Vec3::new(0.0, 0.0, 1.0),
            ambient: 0.0,
            background: 0xFF12_3456,
        };
        let mut fb = Framebuffer::new(16, 16).unwrap();
        let shaded = vis.resolve(&[tri], &materials, &params, &mut fb);
        assert_eq!(shaded, stats.covered_pixels);
        // Facing the light dead-on with ambient 0: pure white.
        assert_eq!(fb.get_pixel(4, 4), Some(0xFFFF_FFFF));
        // Uncovered: exactly the background.
        assert_eq!(fb.get_pixel(13, 13), Some(0xFF12_3456));
    }

    #[test]
    fn resolve_backfacing_surface_gets_only_ambient_and_emissive() {
        let away = VisVertex::screen(2.0, 2.0, 0.5).with_normal(Vec3::new(0.0, 0.0, -1.0));
        let tri = VisTriangle::new(
            [
                away,
                VisVertex::screen(14.0, 2.0, 0.5).with_normal(Vec3::new(0.0, 0.0, -1.0)),
                VisVertex::screen(2.0, 14.0, 0.5).with_normal(Vec3::new(0.0, 0.0, -1.0)),
            ],
            0,
        );
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        vis.rasterize(&[tri]);
        let materials = [VisMaterial {
            base_color: Color::WHITE,
            emissive: Color::rgb(0.1, 0.0, 0.0),
        }];
        let params = ResolveParams {
            light_dir: Vec3::new(0.0, 0.0, 1.0),
            ambient: 0.25,
            background: 0xFF00_0000,
        };
        let mut fb = Framebuffer::new(16, 16).unwrap();
        vis.resolve(&[tri], &materials, &params, &mut fb);
        // Linear components: r = emissive 0.1 + ambient 0.25, g = b =
        // ambient 0.25 (from_argb_u32 returns linear components; the
        // tolerance covers the 8-bit round trip through the framebuffer).
        let px = Color::from_argb_u32(fb.get_pixel(4, 4).unwrap());
        assert!((px.r - 0.35).abs() < 0.02, "r={}", px.r);
        assert!((px.g - 0.25).abs() < 0.02, "g={}", px.g);
        assert!((px.b - 0.25).abs() < 0.02, "b={}", px.b);
    }

    #[test]
    fn resolve_out_of_range_material_uses_magenta_fallback() {
        let mut tri = tri_screen((2.0, 2.0), (14.0, 2.0), (2.0, 14.0), 0.5);
        tri.material = 9;
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        vis.rasterize(&[tri]);
        let params = ResolveParams {
            light_dir: Vec3::new(0.0, 0.0, 1.0),
            ambient: 1.0,
            background: 0xFF00_0000,
        };
        let mut fb = Framebuffer::new(16, 16).unwrap();
        vis.resolve(&[tri], &[], &params, &mut fb);
        assert_eq!(fb.get_pixel(4, 4), Some(Color::MAGENTA.to_argb_u32()));
    }

    #[test]
    fn rasterize_clears_the_previous_frame() {
        let tri = tri_screen((2.0, 2.0), (14.0, 2.0), (2.0, 14.0), 0.5);
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        vis.rasterize(&[tri]);
        assert!(vis.covered_count() > 0);
        let stats = vis.rasterize(&[]);
        assert_eq!(stats.covered_pixels, 0);
        assert_eq!(vis.covered_count(), 0);
    }

    #[test]
    fn random_triangles_keep_the_covered_pixel_invariants() {
        // Deterministic xorshift so the test never flakes.
        let mut state = 0x1234_5678_u32;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state % 2000) as f32 / 100.0 - 2.0 // in [-2, 18)
        };
        let mut tris = Vec::new();
        for i in 0..24 {
            let depth = 0.1 + 0.03 * i as f32;
            tris.push(tri_screen(
                (next(), next()),
                (next(), next()),
                (next(), next()),
                depth,
            ));
        }
        let mut vis = VisibilityBuffer::new(16, 16).unwrap();
        vis.rasterize(&tris);
        for p in vis.as_slice() {
            if p.is_covered() {
                let (b0, b1, b2) = p.bary3();
                assert!(
                    b0 >= -1e-6 && b1 >= -1e-6 && b2 >= -1e-6,
                    "bary=({b0},{b1},{b2})"
                );
                assert!((b0 + b1 + b2 - 1.0).abs() < 1e-4);
                assert!(p.depth.is_finite());
                assert!((p.triangle as usize) < tris.len());
            } else {
                assert_eq!(*p, VisPixel::EMPTY);
            }
        }
    }
}
