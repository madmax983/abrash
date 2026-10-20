#![allow(
    clippy::float_cmp,
    clippy::imprecise_flops,
    clippy::too_long_first_doc_paragraph
)]
//! Multi-channel signed distance field (MSDF / MTSDF) text atlas.
//!
//! This is the CPU counterpart of the `msdf-atlas-gen` / `msdfgen`
//! pipeline (Chlumsky's multi-channel distance fields), rebuilt for
//! Abrash's software rasteriser with no native dependency:
//!
//! 1. A glyph is a [`Shape`]: closed [`Contour`]s of line / quadratic /
//!    cubic edges. [`Shape::color_edges`] assigns each edge an
//!    [`EdgeColor`] whose bits say which RGB channels that edge feeds —
//!    the colour changes at sharp corners and is preserved across
//!    smooth joins, which is the whole trick of MSDF.
//! 2. [`generate_mtsdf`] rasterises one glyph into a small RGBA bitmap:
//!    R, G and B each hold the signed distance to the nearest edge of
//!    that channel, and A holds the *true* signed distance to the
//!    nearest edge of any colour (that fourth channel is what makes it
//!    MTSDF rather than plain MSDF).
//! 3. [`TextAtlas::build`] shelf-packs those bitmaps into one atlas.
//! 4. The "small shader" is [`mtsdf_coverage`] plus
//!    [`GlyphEntry::screen_px_range`] — the CPU form of the canonical
//!    fragment shader:
//!
//!    ```glsl
//!    float sd = median(tex.r, tex.g, tex.b);
//!    float opacity = clamp(screenPxRange * (sd - 0.5) + 0.5, 0.0, 1.0);
//!    ```
//!
//!    [`draw_text`] evaluates it per pixel (bilinear atlas sample,
//!    median, range scaled by the on-screen glyph size) and blends into
//!    a [`Framebuffer`]. Because the range scales with the draw size,
//!    the same atlas stays sharp at 12 px and at 120 px — resolution
//!    independence, with one atlas and no re-rasterisation.
//!
//! # Sign and encoding conventions
//!
//! Distances follow [`crate::sdf`]: **negative inside, positive
//! outside**. A distance `d` measured in atlas pixels is encoded as
//! `0.5 - d / px_range`, clamped to `[0, 1]`, so the edge sits at
//! exactly `0.5`, deep interior approaches `1`, and deep exterior `0`.
//! Plane coordinates (glyph space) are **y-up** with the baseline at
//! `y = 0`, like font outlines; screen coordinates are y-down, and
//! [`draw_text`] flips between them.
//!
//! # Invariants (the spec these functions are tested against)
//!
//! 1. `median3` of a texel on the outline is `0.5`; the median is
//!    inside (`> 0.5`) exactly where the shape is inside near smooth
//!    edges, and — unlike the true-distance alpha channel — it keeps
//!    sharp corners sharp: just outside a corner, the median distance
//!    is the distance to the corner's edge *planes*, not the rounded
//!    distance to the corner point.
//! 2. Coverage is in `[0, 1]`, is `0.5` when the reconstructed
//!    distance is `0`, and is monotonically non-decreasing in the
//!    reconstructed distance.
//! 3. Atlas packing is deterministic, rectangles never overlap, and
//!    every glyph rectangle lies inside the atlas.
//! 4. Rendering never touches a pixel whose coverage is `0`, and a
//!    pixel deep inside a glyph takes the draw colour exactly.
//!
//! # Honest limitations
//!
//! Distances are true orthogonal distances to the nearest edge point,
//! not msdfgen's pseudo-distances (which project along the edge
//! normal). The median reconstruction is exact on edges and at
//! corners either way; pseudo-distance only changes the field *far*
//! from the outline, outside the `px_range` band that the shader
//! reads. Edge colouring implements the corner rule (change colour
//! when the direction change at a vertex exceeds a threshold); it
//! does not implement msdfgen's teardrop special cases.

use std::collections::BTreeMap;

use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::math::Vec2;

/// Default distance range, in atlas pixels, stored in the field.
/// Matches the `msdf-atlas-gen` default of 4 (called `pxRange` there;
/// larger ranges survive minification better, smaller ranges are
/// crisper at the design size).
pub const DEFAULT_PX_RANGE: f32 = 4.0;

/// Default direction-change angle, in degrees, at or above which a
/// vertex counts as a corner and the edge colour changes.
pub const DEFAULT_CORNER_THRESHOLD_DEG: f32 = 60.0;

// ── Edge colours ─────────────────────────────────────────────────────────────

/// Which distance-field channels an edge contributes to.
///
/// The value is a bitset over (R, G, B): `Yellow` feeds R and G,
/// `White` feeds all three, and so on — the same colour names and
/// channel assignments as `msdfgen`.
///
/// # Examples
///
/// ```
/// use abrash_core::msdf::EdgeColor;
///
/// assert_eq!(EdgeColor::Yellow.channel_mask(), 0b011);
/// assert!(EdgeColor::Magenta.includes_channel(0)); // R
/// assert!(!EdgeColor::Magenta.includes_channel(1)); // not G
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EdgeColor {
    /// Feeds no channel (isolated edges; reconstructed from alpha only).
    Black,
    /// Feeds R only.
    Red,
    /// Feeds G only.
    Green,
    /// Feeds B only.
    Blue,
    /// Feeds R and G.
    Yellow,
    /// Feeds R and B.
    Magenta,
    /// Feeds G and B.
    Cyan,
    /// Feeds R, G and B.
    White,
}

impl EdgeColor {
    /// Channel bitset: bit 0 = R, bit 1 = G, bit 2 = B.
    #[must_use]
    pub const fn channel_mask(self) -> u8 {
        match self {
            Self::Black => 0,
            Self::Red => 0b001,
            Self::Green => 0b010,
            Self::Yellow => 0b011,
            Self::Blue => 0b100,
            Self::Magenta => 0b101,
            Self::Cyan => 0b110,
            Self::White => 0b111,
        }
    }

    /// Whether this colour feeds channel `channel` (0 = R, 1 = G, 2 = B).
    #[must_use]
    pub const fn includes_channel(self, channel: usize) -> bool {
        channel < 3 && (self.channel_mask() >> channel) & 1 == 1
    }
}

/// The colour cycle used at corners: consecutive corner runs always
/// take the next colour, so the two edges meeting at a corner differ
/// in at least one channel.
const CORNER_CYCLE: [EdgeColor; 3] = [EdgeColor::Yellow, EdgeColor::Cyan, EdgeColor::Magenta];

// ── Edge segments ────────────────────────────────────────────────────────────

/// One outline edge: a line, a quadratic Bezier, or a cubic Bezier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeSegment {
    /// Straight segment from `from` to `to`.
    Line {
        /// Start point.
        from: Vec2,
        /// End point.
        to: Vec2,
    },
    /// Quadratic Bezier with one control point.
    Quadratic {
        /// Start point.
        from: Vec2,
        /// Control point.
        ctrl: Vec2,
        /// End point.
        to: Vec2,
    },
    /// Cubic Bezier with two control points.
    Cubic {
        /// Start point.
        from: Vec2,
        /// First control point.
        ctrl1: Vec2,
        /// Second control point.
        ctrl2: Vec2,
        /// End point.
        to: Vec2,
    },
}

impl EdgeSegment {
    /// The start point of the segment.
    #[must_use]
    pub const fn start(self) -> Vec2 {
        match self {
            Self::Line { from, .. } | Self::Quadratic { from, .. } | Self::Cubic { from, .. } => {
                from
            }
        }
    }

    /// The end point of the segment.
    #[must_use]
    pub const fn end(self) -> Vec2 {
        match self {
            Self::Line { to, .. } | Self::Quadratic { to, .. } | Self::Cubic { to, .. } => to,
        }
    }

    /// The point on the segment at parameter `t` in `[0, 1]`.
    #[must_use]
    pub fn point(self, t: f32) -> Vec2 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Line { from, to } => from.lerp(to, t),
            Self::Quadratic { from, ctrl, to } => {
                let u = 1.0 - t;
                from * (u * u) + ctrl * (2.0 * u * t) + to * (t * t)
            }
            Self::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => {
                let u = 1.0 - t;
                from * (u * u * u)
                    + ctrl1 * (3.0 * u * u * t)
                    + ctrl2 * (3.0 * u * t * t)
                    + to * (t * t * t)
            }
        }
    }

    /// The (unnormalised) tangent at parameter `t`.
    #[must_use]
    fn derivative(self, t: f32) -> Vec2 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Line { from, to } => to - from,
            Self::Quadratic { from, ctrl, to } => {
                (ctrl - from) * (2.0 * (1.0 - t)) + (to - ctrl) * (2.0 * t)
            }
            Self::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => {
                let u = 1.0 - t;
                (ctrl1 - from) * (3.0 * u * u)
                    + (ctrl2 - ctrl1) * (6.0 * u * t)
                    + (to - ctrl2) * (3.0 * t * t)
            }
        }
    }

    /// Tangent direction at the start, falling back to the chord for
    /// degenerate control points.
    fn start_tangent(self) -> Vec2 {
        let d = self.derivative(0.0);
        if d.length_sq() > 1e-20 {
            d
        } else {
            self.end() - self.start()
        }
    }

    /// Tangent direction at the end, falling back to the chord.
    fn end_tangent(self) -> Vec2 {
        let d = self.derivative(1.0);
        if d.length_sq() > 1e-20 {
            d
        } else {
            self.end() - self.start()
        }
    }

    /// The parameter of the point on the segment closest to `p`.
    ///
    /// Lines are solved exactly. Beziers are minimised by dense
    /// sampling followed by golden-section refinement of the bracket
    /// around the best sample — deterministic, and accurate to far
    /// below one atlas texel at any sane glyph size.
    #[must_use]
    pub fn closest_parameter(self, p: Vec2) -> f32 {
        if let Self::Line { from, to } = self {
            let ab = to - from;
            let denom = ab.dot(ab);
            if denom <= 1e-20 {
                return 0.0;
            }
            return ((p - from).dot(ab) / denom).clamp(0.0, 1.0);
        }
        {
            const SAMPLES: usize = 24;
            let mut best_t = 0.0_f32;
            let mut best_d = f32::INFINITY;
            for i in 0..=SAMPLES {
                let t = i as f32 / SAMPLES as f32;
                let d = self.point(t).distance_sq(p);
                if d < best_d {
                    best_d = d;
                    best_t = t;
                }
            }
            let step = 1.0 / SAMPLES as f32;
            let mut lo = (best_t - step).max(0.0);
            let mut hi = (best_t + step).min(1.0);
            // Golden-section search on the bracketed minimum.
            let inv_phi = 0.618_034_f32;
            let mut c = hi - inv_phi * (hi - lo);
            let mut d = lo + inv_phi * (hi - lo);
            for _ in 0..40 {
                if self.point(c).distance_sq(p) < self.point(d).distance_sq(p) {
                    hi = d;
                    d = c;
                    c = hi - inv_phi * (hi - lo);
                } else {
                    lo = c;
                    c = d;
                    d = lo + inv_phi * (hi - lo);
                }
            }
            f32::midpoint(lo, hi)
        }
    }

    /// Unsigned distance from `p` to the segment.
    #[must_use]
    pub fn distance(self, p: Vec2) -> f32 {
        self.point(self.closest_parameter(p)).distance(p)
    }

    /// Axis-aligned bounds of the segment (control-point hull for
    /// Beziers, which contains the curve).
    #[must_use]
    pub fn bounds(self) -> Bounds {
        match self {
            Self::Line { from, to } => Bounds::from_points(&[from, to]),
            Self::Quadratic { from, ctrl, to } => Bounds::from_points(&[from, ctrl, to]),
            Self::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => Bounds::from_points(&[from, ctrl1, ctrl2, to]),
        }
    }

    /// Approximate the segment with `subdivisions` line pieces
    /// (endpoints included) for point-in-shape tests.
    fn flatten(self, subdivisions: usize, out: &mut Vec<Vec2>) {
        if matches!(self, Self::Line { .. }) {
            out.push(self.end());
            return;
        }
        for i in 1..=subdivisions {
            out.push(self.point(i as f32 / subdivisions as f32));
        }
    }
}

/// An [`EdgeSegment`] plus the channels it feeds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColoredEdge {
    /// The geometry.
    pub segment: EdgeSegment,
    /// The channels this edge contributes distances to.
    pub color: EdgeColor,
}

impl ColoredEdge {
    /// A line edge with the given colour.
    #[must_use]
    pub const fn line(from: Vec2, to: Vec2, color: EdgeColor) -> Self {
        Self {
            segment: EdgeSegment::Line { from, to },
            color,
        }
    }

    /// A quadratic Bezier edge with the given colour.
    #[must_use]
    pub const fn quadratic(from: Vec2, ctrl: Vec2, to: Vec2, color: EdgeColor) -> Self {
        Self {
            segment: EdgeSegment::Quadratic { from, ctrl, to },
            color,
        }
    }

    /// A cubic Bezier edge with the given colour.
    #[must_use]
    pub const fn cubic(from: Vec2, ctrl1: Vec2, ctrl2: Vec2, to: Vec2, color: EdgeColor) -> Self {
        Self {
            segment: EdgeSegment::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            },
            color,
        }
    }
}

// ── Bounds ───────────────────────────────────────────────────────────────────

/// An axis-aligned rectangle in plane (glyph) space, y-up:
/// `left <= right`, `bottom <= top`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Smallest x.
    pub left: f32,
    /// Smallest y (the bottom, since plane space is y-up).
    pub bottom: f32,
    /// Largest x.
    pub right: f32,
    /// Largest y.
    pub top: f32,
}

impl Bounds {
    /// Bounds covering all of `points` (degenerate at the origin for
    /// an empty slice).
    #[must_use]
    pub fn from_points(points: &[Vec2]) -> Self {
        let mut b = Self {
            left: 0.0,
            bottom: 0.0,
            right: 0.0,
            top: 0.0,
        };
        if let Some(first) = points.first() {
            b = Self {
                left: first.x,
                bottom: first.y,
                right: first.x,
                top: first.y,
            };
            for p in &points[1..] {
                b.left = b.left.min(p.x);
                b.bottom = b.bottom.min(p.y);
                b.right = b.right.max(p.x);
                b.top = b.top.max(p.y);
            }
        }
        b
    }

    /// Width (`right - left`).
    #[must_use]
    pub fn width(self) -> f32 {
        self.right - self.left
    }

    /// Height (`top - bottom`).
    #[must_use]
    pub fn height(self) -> f32 {
        self.top - self.bottom
    }

    /// The bounds grown by `amount` on every side.
    #[must_use]
    pub fn expanded(self, amount: f32) -> Self {
        Self {
            left: self.left - amount,
            bottom: self.bottom - amount,
            right: self.right + amount,
            top: self.top + amount,
        }
    }

    /// The smallest bounds containing both `self` and `other`.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            bottom: self.bottom.min(other.bottom),
            right: self.right.max(other.right),
            top: self.top.max(other.top),
        }
    }
}

// ── Shapes ───────────────────────────────────────────────────────────────────

/// One closed outline loop. Edges are assumed to connect end-to-start
/// and the last edge to close back to the first edge's start.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Contour {
    /// The edges, in traversal order.
    pub edges: Vec<ColoredEdge>,
}

impl Contour {
    /// A contour through `points` as straight edges of one colour,
    /// closed back to the first point.
    #[must_use]
    pub fn polygon(points: &[Vec2], color: EdgeColor) -> Self {
        let mut edges = Vec::with_capacity(points.len());
        for i in 0..points.len() {
            edges.push(ColoredEdge::line(
                points[i],
                points[(i + 1) % points.len()],
                color,
            ));
        }
        Self { edges }
    }

    /// Bounds over all edges.
    #[must_use]
    pub fn bounds(&self) -> Bounds {
        self.edges
            .iter()
            .map(|e| e.segment.bounds())
            .reduce(Bounds::union)
            .unwrap_or(Bounds {
                left: 0.0,
                bottom: 0.0,
                right: 0.0,
                top: 0.0,
            })
    }
}

/// A glyph outline: a set of closed contours. Interior is decided by
/// the even-odd rule across all contours, so holes are just extra
/// contours inside another one, regardless of winding.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Shape {
    /// The outline loops (outer loops and holes alike).
    pub contours: Vec<Contour>,
}

impl Shape {
    /// Bounds over all contours, or `None` for an empty shape.
    #[must_use]
    pub fn bounds(&self) -> Option<Bounds> {
        self.contours
            .iter()
            .map(Contour::bounds)
            .reduce(Bounds::union)
    }

    /// Whether `p` is inside the shape (even-odd ray cast over the
    /// flattened outlines).
    #[must_use]
    pub fn is_inside(&self, p: Vec2) -> bool {
        let mut inside = false;
        for contour in &self.contours {
            if contour.edges.is_empty() {
                continue;
            }
            let mut poly: Vec<Vec2> = vec![contour.edges[0].segment.start()];
            for edge in &contour.edges {
                edge.segment.flatten(16, &mut poly);
            }
            for w in poly.windows(2) {
                let (a, b) = (w[0], w[1]);
                if (a.y > p.y) != (b.y > p.y) {
                    let x_cross = a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y);
                    if p.x < x_cross {
                        inside = !inside;
                    }
                }
            }
        }
        inside
    }

    fn sign(&self, p: Vec2) -> f32 {
        if self.is_inside(p) { -1.0 } else { 1.0 }
    }

    /// Signed true distance from `p` to the outline: the distance to
    /// the nearest edge of any colour, negative inside. This is the
    /// value stored in the MTSDF alpha channel.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::math::Vec2;
    /// use abrash_core::msdf::{Contour, EdgeColor, Shape};
    ///
    /// let square = Shape { contours: vec![Contour::polygon(&[
    ///     Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0),
    ///     Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0),
    /// ], EdgeColor::White)] };
    /// assert!((square.true_distance(Vec2::new(0.5, 0.5)) + 0.5).abs() < 1e-6);
    /// assert!((square.true_distance(Vec2::new(2.0, 0.5)) - 1.0).abs() < 1e-6);
    /// ```
    #[must_use]
    pub fn true_distance(&self, p: Vec2) -> f32 {
        let best = self.unsigned_true_distance(p);
        if best.is_infinite() {
            return 0.0;
        }
        self.sign(p) * best
    }

    fn unsigned_true_distance(&self, p: Vec2) -> f32 {
        let mut best = f32::INFINITY;
        for contour in &self.contours {
            for edge in &contour.edges {
                best = best.min(edge.segment.distance(p));
            }
        }
        best
    }

    fn unsigned_channel_distance(&self, p: Vec2, channel: usize) -> f32 {
        let mut best = f32::INFINITY;
        for contour in &self.contours {
            for edge in &contour.edges {
                if edge.color.includes_channel(channel) {
                    best = best.min(edge.segment.distance(p));
                }
            }
        }
        if best.is_infinite() {
            best = self.unsigned_true_distance(p);
        }
        best
    }

    /// Signed distance from `p` to the nearest edge feeding
    /// `channel` (0 = R, 1 = G, 2 = B). If no edge feeds the channel,
    /// falls back to the true distance, so a single-colour shape
    /// produces the same field in every channel it *does* define and
    /// a sane field in the rest.
    #[must_use]
    pub fn channel_distance(&self, p: Vec2, channel: usize) -> f32 {
        let best = self.unsigned_channel_distance(p, channel);
        if best.is_infinite() {
            return 0.0;
        }
        self.sign(p) * best
    }

    /// Assign edge colours by the corner rule: walk each contour
    /// keeping the current colour across smooth vertices and taking
    /// the next colour in the cycle at corners (direction change at
    /// or above `corner_threshold_deg`). A one-edge contour and the
    /// first edge of every contour restart the cycle, and a closing
    /// corner whose two runs collide is recoloured to a colour
    /// distinct from both neighbours.
    pub fn color_edges(&mut self, corner_threshold_deg: f32) {
        for contour in &mut self.contours {
            color_contour(contour, corner_threshold_deg);
        }
    }

    /// A copy of this shape with [`Shape::color_edges`] applied.
    #[must_use]
    pub fn colored(mut self, corner_threshold_deg: f32) -> Self {
        self.color_edges(corner_threshold_deg);
        self
    }
}

/// Direction change at the vertex between two segments, in degrees:
/// 0 for a straight continuation, 180 for a full reversal.
fn turn_degrees(incoming: Vec2, outgoing: Vec2) -> f32 {
    let a = incoming.normalize_or_zero();
    let b = outgoing.normalize_or_zero();
    if a == Vec2::ZERO || b == Vec2::ZERO {
        return 0.0;
    }
    a.dot(b).clamp(-1.0, 1.0).acos().to_degrees()
}

fn color_contour(contour: &mut Contour, corner_threshold_deg: f32) {
    let n = contour.edges.len();
    if n == 0 {
        return;
    }
    if n == 1 {
        contour.edges[0].color = EdgeColor::White;
        return;
    }
    // Corner flags at each edge's start vertex.
    let mut corner_at: Vec<bool> = vec![false; n];
    for i in 0..n {
        let prev = contour.edges[(i + n - 1) % n].segment;
        let next = contour.edges[i].segment;
        corner_at[i] =
            turn_degrees(prev.end_tangent(), next.start_tangent()) >= corner_threshold_deg;
    }
    let mut cycle_idx = 0_usize;
    let mut current = CORNER_CYCLE[0];
    for i in 0..n {
        if i > 0 && corner_at[i] {
            cycle_idx = (cycle_idx + 1) % CORNER_CYCLE.len();
            current = CORNER_CYCLE[cycle_idx];
        }
        contour.edges[i].color = current;
    }
    // Closing vertex: if it is a corner and both runs share a colour,
    // recolour the last run to a colour distinct from both sides.
    if corner_at[0] && contour.edges[n - 1].color == contour.edges[0].color {
        let before_last = contour.edges[n - 2].color;
        let first = contour.edges[0].color;
        for candidate in [
            EdgeColor::Yellow,
            EdgeColor::Cyan,
            EdgeColor::Magenta,
            EdgeColor::White,
            EdgeColor::Red,
            EdgeColor::Green,
            EdgeColor::Blue,
        ] {
            if candidate != before_last && candidate != first {
                // Recolour only the final smooth run (back to its corner).
                let mut i = n - 1;
                loop {
                    contour.edges[i].color = candidate;
                    if i == 0 || corner_at[i] {
                        break;
                    }
                    i -= 1;
                }
                break;
            }
        }
    }
}

// ── Distance encoding and the shader core ────────────────────────────────────

/// Encode a signed distance (in atlas pixels, negative inside) as a
/// field value in `[0, 1]`: `0.5` on the edge, saturating `px_range`
/// pixels to either side.
///
/// # Examples
///
/// ```
/// use abrash_core::msdf::encode_distance;
///
/// assert_eq!(encode_distance(0.0, 4.0), 0.5);
/// assert_eq!(encode_distance(-4.0, 4.0), 1.0); // one range inside
/// assert_eq!(encode_distance(4.0, 4.0), 0.0); // one range outside
/// assert_eq!(encode_distance(-100.0, 4.0), 1.0); // saturates
/// ```
#[must_use]
pub fn encode_distance(signed_dist_px: f32, px_range: f32) -> f32 {
    if !(px_range > 0.0) {
        return if signed_dist_px < 0.0 {
            1.0
        } else if signed_dist_px > 0.0 {
            0.0
        } else {
            0.5
        };
    }
    (0.5 - signed_dist_px / px_range).clamp(0.0, 1.0)
}

/// The median of three values — the MSDF reconstruction function.
/// For a point on the outline, at least two channels are at their
/// edge, so the median recovers the true edge distance even where
/// individual channels measure a *different* edge's distance.
///
/// # Examples
///
/// ```
/// use abrash_core::msdf::median3;
///
/// assert_eq!(median3(0.2, 0.5, 0.9), 0.5);
/// assert_eq!(median3(0.9, 0.5, 0.2), 0.5);
/// assert_eq!(median3(1.0, 1.0, 0.0), 1.0);
/// ```
#[must_use]
pub const fn median3(a: f32, b: f32, c: f32) -> f32 {
    // median = max(min(a, b), min(max(a, b), c))
    a.min(b).max(a.max(b).min(c))
}

/// The reconstructed signed distance (in encoded units, centred on
/// `0.5`) of an MTSDF texel: the median of its RGB channels.
#[must_use]
pub const fn mtsdf_distance(texel: [f32; 4]) -> f32 {
    median3(texel[0], texel[1], texel[2])
}

/// Coverage from an MTSDF sample — the canonical MSDF shader,
/// CPU edition:
///
/// ```text
/// sd      = median(r, g, b)
/// opacity = clamp(screen_px_range * (sd - 0.5) + 0.5, 0, 1)
/// ```
///
/// `screen_px_range` is the atlas `px_range` re-expressed in screen
/// pixels at the current draw size (see
/// [`GlyphEntry::screen_px_range`]); it is what makes the edge stay
/// one screen pixel wide at every scale.
///
/// # Examples
///
/// ```
/// use abrash_core::msdf::mtsdf_coverage;
///
/// assert_eq!(mtsdf_coverage([0.5, 0.5, 0.5, 0.5], 4.0), 0.5); // on the edge
/// assert_eq!(mtsdf_coverage([1.0, 1.0, 1.0, 1.0], 4.0), 1.0); // deep inside
/// assert_eq!(mtsdf_coverage([0.0, 0.0, 0.0, 0.0], 4.0), 0.0); // deep outside
/// ```
#[must_use]
pub fn mtsdf_coverage(texel: [f32; 4], screen_px_range: f32) -> f32 {
    let sd = mtsdf_distance(texel);
    (screen_px_range.max(0.0) * (sd - 0.5) + 0.5).clamp(0.0, 1.0)
}

fn encode_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

// ── Glyph generation ─────────────────────────────────────────────────────────

/// Whether the alpha channel carries the true distance (MTSDF) or is
/// left fully opaque (plain MSDF).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtlasKind {
    /// RGB only; alpha is 255 everywhere.
    Msdf,
    /// RGB plus the true signed distance in alpha.
    Mtsdf,
}

/// One generated glyph bitmap, before atlas packing.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphBitmap {
    /// Bitmap width in texels.
    pub width: u32,
    /// Bitmap height in texels.
    pub height: u32,
    /// Row-major RGBA texels, top row first.
    pub rgba: Vec<u8>,
    /// Plane-space rectangle the bitmap covers (ink bounds already
    /// expanded by the distance range).
    pub plane_bounds: Bounds,
    /// Distance range in atlas pixels used for the encoding.
    pub px_range: f32,
    /// Which variant was generated.
    pub kind: AtlasKind,
}

impl GlyphBitmap {
    /// One texel as normalised `[r, g, b, a]` in `[0, 1]`.
    /// Out-of-range coordinates clamp to the nearest edge texel.
    #[must_use]
    pub fn texel(&self, x: i32, y: i32) -> [f32; 4] {
        if self.width == 0 || self.height == 0 {
            return [0.0; 4];
        }
        let cx = x.clamp(0, self.width as i32 - 1) as usize;
        let cy = y.clamp(0, self.height as i32 - 1) as usize;
        let idx = (cy * self.width as usize + cx) * 4;
        [
            f32::from(self.rgba[idx]) / 255.0,
            f32::from(self.rgba[idx + 1]) / 255.0,
            f32::from(self.rgba[idx + 2]) / 255.0,
            f32::from(self.rgba[idx + 3]) / 255.0,
        ]
    }

    /// Bilinear sample at floating-point texel coordinates
    /// (`(0, 0)` is the centre of the top-left texel).
    #[must_use]
    pub fn sample_bilinear(&self, x: f32, y: f32) -> [f32; 4] {
        bilinear(
            |tx, ty| self.texel(tx, ty),
            x,
            y,
            self.width as i32,
            self.height as i32,
        )
    }
}

/// Generation settings shared by direct bitmap generation and atlas
/// building.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneratorConfig {
    /// Distance range in atlas pixels.
    pub px_range: f32,
    /// Atlas texels per plane unit at generation time.
    pub pixels_per_unit: f32,
    /// MSDF or MTSDF output.
    pub kind: AtlasKind,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            px_range: DEFAULT_PX_RANGE,
            pixels_per_unit: 32.0,
            kind: AtlasKind::Mtsdf,
        }
    }
}

/// Generate one glyph's MSDF/MTSDF bitmap.
///
/// `plane_bounds` is the rectangle of glyph space to rasterise
/// (usually the shape's bounds expanded by `px_range /
/// pixels_per_unit` — [`TextAtlas::build`] does that for you). Texel
/// centres map linearly onto it, with the top row at `plane_bounds.top`
/// because plane space is y-up and bitmaps are stored top-first.
/// Distances are converted to atlas pixels with `pixels_per_unit`
/// before encoding, so the bitmap and the plane bounds must come from
/// the same scale — passing mismatched values silently stretches the
/// field's range, exactly as in the GPU pipeline.
#[must_use]
pub fn generate_mtsdf(
    shape: &Shape,
    width: u32,
    height: u32,
    plane_bounds: Bounds,
    config: GeneratorConfig,
) -> GlyphBitmap {
    let mut rgba = vec![0_u8; width as usize * height as usize * 4];
    let plane_w = plane_bounds.width().max(1e-9);
    let plane_h = plane_bounds.height().max(1e-9);
    for ty in 0..height {
        for tx in 0..width {
            let p = Vec2::new(
                plane_bounds.left + (tx as f32 + 0.5) / width as f32 * plane_w,
                plane_bounds.top - (ty as f32 + 0.5) / height as f32 * plane_h,
            );
            let idx = (ty as usize * width as usize + tx as usize) * 4;
            // One inside/outside test per texel, shared by all channels.
            let sign = shape.sign(p);
            for (ch, slot) in rgba[idx..idx + 3].iter_mut().enumerate() {
                let d_px = sign * shape.unsigned_channel_distance(p, ch) * config.pixels_per_unit;
                *slot = encode_byte(encode_distance(d_px, config.px_range));
            }
            rgba[idx + 3] = match config.kind {
                AtlasKind::Msdf => 255,
                AtlasKind::Mtsdf => encode_byte(encode_distance(
                    sign * shape.unsigned_true_distance(p) * config.pixels_per_unit,
                    config.px_range,
                )),
            };
        }
    }
    GlyphBitmap {
        width,
        height,
        rgba,
        plane_bounds,
        px_range: config.px_range,
        kind: config.kind,
    }
}

// ── The atlas ────────────────────────────────────────────────────────────────

/// A glyph to place in an atlas: its outline, horizontal advance, all
/// in plane units (em-relative for real fonts; the demo glyphs use a
/// 1-unit cap height).
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphSource {
    /// The character this glyph belongs to.
    pub key: char,
    /// The outline.
    pub shape: Shape,
    /// Horizontal advance in plane units.
    pub advance: f32,
}

/// Where one glyph ended up in the atlas, with everything the shader
/// and the layout engine need.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphEntry {
    /// Atlas x of the glyph bitmap's left edge, in texels.
    pub atlas_x: u32,
    /// Atlas y of the glyph bitmap's top edge, in texels.
    pub atlas_y: u32,
    /// Bitmap width in texels.
    pub width: u32,
    /// Bitmap height in texels.
    pub height: u32,
    /// Plane-space rectangle the bitmap covers.
    pub plane_bounds: Bounds,
    /// Horizontal advance in plane units.
    pub advance: f32,
    /// Distance range in atlas pixels used for this glyph.
    pub px_range: f32,
}

impl GlyphEntry {
    /// The atlas `px_range` re-expressed in screen pixels when the
    /// glyph is drawn at `size_px` screen pixels per plane unit:
    /// the range scales with how many screen pixels one plane unit
    /// (and therefore one atlas texel) covers.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::msdf::{Bounds, GlyphEntry};
    ///
    /// let entry = GlyphEntry {
    ///     atlas_x: 0, atlas_y: 0, width: 32, height: 32,
    ///     plane_bounds: Bounds { left: 0.0, bottom: 0.0, right: 1.0, top: 1.0 },
    ///     advance: 1.0, px_range: 4.0,
    /// };
    /// // 32 atlas texels per unit drawn at 32 px/unit: range stays 4.
    /// assert!((entry.screen_px_range(32.0) - 4.0).abs() < 1e-5);
    /// // Drawn four times larger, the same range covers 16 screen px.
    /// assert!((entry.screen_px_range(128.0) - 16.0).abs() < 1e-4);
    /// ```
    #[must_use]
    pub fn screen_px_range(self, size_px: f32) -> f32 {
        let plane_w = self.plane_bounds.width();
        if plane_w <= 0.0 || self.width == 0 {
            return self.px_range;
        }
        let atlas_px_per_unit = self.width as f32 / plane_w;
        self.px_range * size_px / atlas_px_per_unit
    }
}

/// A packed MSDF/MTSDF atlas plus its glyph directory.
#[derive(Debug, Clone, PartialEq)]
pub struct TextAtlas {
    /// Atlas width in texels.
    pub width: u32,
    /// Atlas height in texels.
    pub height: u32,
    /// Row-major RGBA texels, top row first.
    pub rgba: Vec<u8>,
    /// Glyph directory by character.
    pub glyphs: BTreeMap<char, GlyphEntry>,
    /// Which variant the atlas holds.
    pub kind: AtlasKind,
}

impl TextAtlas {
    /// Build an atlas from `sources`: colour each shape's edges,
    /// generate each glyph bitmap at `config.pixels_per_unit` with
    /// its ink bounds expanded by the distance range, then shelf-pack
    /// the bitmaps (tallest first, ties by character) into an atlas
    /// `atlas_width` texels wide. Empty shapes (like a space) get a
    /// zero-size entry that draws nothing but still advances.
    #[must_use]
    pub fn build(sources: &[GlyphSource], config: GeneratorConfig, atlas_width: u32) -> Self {
        let atlas_width = atlas_width.max(1);
        let pad_plane = if config.pixels_per_unit > 0.0 {
            config.px_range / config.pixels_per_unit
        } else {
            0.0
        };
        // Generate bitmaps first so packing knows every size.
        let mut bitmaps: Vec<(char, f32, GlyphBitmap)> = Vec::with_capacity(sources.len());
        for src in sources {
            let colored = src.shape.clone().colored(DEFAULT_CORNER_THRESHOLD_DEG);
            let Some(ink) = colored.bounds() else {
                bitmaps.push((
                    src.key,
                    src.advance,
                    GlyphBitmap {
                        width: 0,
                        height: 0,
                        rgba: Vec::new(),
                        plane_bounds: Bounds {
                            left: 0.0,
                            bottom: 0.0,
                            right: 0.0,
                            top: 0.0,
                        },
                        px_range: config.px_range,
                        kind: config.kind,
                    },
                ));
                continue;
            };
            let bounds = ink.expanded(pad_plane);
            let w = (bounds.width() * config.pixels_per_unit).ceil().max(1.0) as u32;
            let h = (bounds.height() * config.pixels_per_unit).ceil().max(1.0) as u32;
            let bmp = generate_mtsdf(&colored, w, h, bounds, config);
            bitmaps.push((src.key, src.advance, bmp));
        }
        // Shelf pack: tallest bitmaps first so shelves stay shallow.
        let mut order: Vec<usize> = (0..bitmaps.len()).collect();
        order.sort_by(|&a, &b| {
            bitmaps[b]
                .2
                .height
                .cmp(&bitmaps[a].2.height)
                .then(bitmaps[a].0.cmp(&bitmaps[b].0))
        });
        let mut placements: Vec<(u32, u32)> = vec![(0, 0); bitmaps.len()];
        let mut shelf_y = 0_u32;
        let mut shelf_h = 0_u32;
        let mut cursor_x = 0_u32;
        for &i in &order {
            let bmp = &bitmaps[i].2;
            if bmp.width == 0 || bmp.height == 0 {
                continue;
            }
            if cursor_x > 0 && cursor_x + bmp.width > atlas_width {
                shelf_y += shelf_h;
                shelf_h = 0;
                cursor_x = 0;
            }
            placements[i] = (cursor_x, shelf_y);
            cursor_x += bmp.width;
            shelf_h = shelf_h.max(bmp.height);
        }
        let total_h = shelf_y + shelf_h;
        let mut atlas = Self {
            width: atlas_width,
            height: total_h,
            rgba: vec![0_u8; atlas_width as usize * total_h as usize * 4],
            glyphs: BTreeMap::new(),
            kind: config.kind,
        };
        for (i, (key, advance, bmp)) in bitmaps.iter().enumerate() {
            let (ax, ay) = placements[i];
            if bmp.width > 0 && bmp.height > 0 {
                for row in 0..bmp.height {
                    let src_off = row as usize * bmp.width as usize * 4;
                    let dst_off = ((ay + row) as usize * atlas_width as usize + ax as usize) * 4;
                    atlas.rgba[dst_off..dst_off + bmp.width as usize * 4]
                        .copy_from_slice(&bmp.rgba[src_off..src_off + bmp.width as usize * 4]);
                }
            }
            atlas.glyphs.insert(
                *key,
                GlyphEntry {
                    atlas_x: ax,
                    atlas_y: ay,
                    width: bmp.width,
                    height: bmp.height,
                    plane_bounds: bmp.plane_bounds,
                    advance: *advance,
                    px_range: bmp.px_range,
                },
            );
        }
        atlas
    }

    /// Directory lookup for one character.
    #[must_use]
    pub fn glyph(&self, key: char) -> Option<&GlyphEntry> {
        self.glyphs.get(&key)
    }

    /// One atlas texel as normalised `[r, g, b, a]`, clamped to edges.
    #[must_use]
    pub fn texel(&self, x: i32, y: i32) -> [f32; 4] {
        if self.width == 0 || self.height == 0 {
            return [0.0; 4];
        }
        let cx = x.clamp(0, self.width as i32 - 1) as usize;
        let cy = y.clamp(0, self.height as i32 - 1) as usize;
        let idx = (cy * self.width as usize + cx) * 4;
        [
            f32::from(self.rgba[idx]) / 255.0,
            f32::from(self.rgba[idx + 1]) / 255.0,
            f32::from(self.rgba[idx + 2]) / 255.0,
            f32::from(self.rgba[idx + 3]) / 255.0,
        ]
    }

    /// Bilinear sample of one glyph at glyph-local `uv` in `[0, 1]`
    /// (`(0, 0)` is the bitmap's top-left corner, matching the stored
    /// row order). Returns `None` for an unknown or empty glyph.
    #[must_use]
    pub fn sample_glyph(&self, key: char, uv: Vec2) -> Option<[f32; 4]> {
        let entry = self.glyph(key)?;
        if entry.width == 0 || entry.height == 0 {
            return None;
        }
        let lx = uv.x.clamp(0.0, 1.0) * (entry.width as f32 - 1.0);
        let ly = uv.y.clamp(0.0, 1.0) * (entry.height as f32 - 1.0);
        Some(bilinear(
            |tx, ty| self.texel(tx + entry.atlas_x as i32, ty + entry.atlas_y as i32),
            lx,
            ly,
            entry.width as i32,
            entry.height as i32,
        ))
    }
}

/// Shared bilinear interpolation over a texel-fetch closure.
fn bilinear(
    fetch: impl Fn(i32, i32) -> [f32; 4],
    x: f32,
    y: f32,
    width: i32,
    height: i32,
) -> [f32; 4] {
    if width <= 0 || height <= 0 {
        return [0.0; 4];
    }
    let fx = x.clamp(0.0, (width - 1) as f32);
    let fy = y.clamp(0.0, (height - 1) as f32);
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let c00 = fetch(x0, y0);
    let c10 = fetch(x0 + 1, y0);
    let c01 = fetch(x0, y0 + 1);
    let c11 = fetch(x0 + 1, y0 + 1);
    let mut out = [0.0_f32; 4];
    for (i, slot) in out.iter_mut().enumerate() {
        let top = c00[i] + (c10[i] - c00[i]) * tx;
        let bottom = c01[i] + (c11[i] - c01[i]) * tx;
        *slot = top + (bottom - top) * ty;
    }
    out
}

// ── Rendering: the shader, evaluated per pixel ───────────────────────────────

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

/// Draw one glyph with its baseline origin at screen point `origin`
/// (y-down screen space; `origin` is where plane `(0, 0)` lands),
/// at `size_px` screen pixels per plane unit. Returns the advance in
/// screen pixels, or `None` for an unknown glyph.
pub fn draw_glyph(
    fb: &mut Framebuffer,
    atlas: &TextAtlas,
    key: char,
    origin: Vec2,
    size_px: f32,
    color: Color,
) -> Option<f32> {
    let entry = *atlas.glyph(key)?;
    if entry.width == 0 || entry.height == 0 || size_px <= 0.0 {
        return Some(entry.advance * size_px.max(0.0));
    }
    let b = entry.plane_bounds;
    // Screen-space rectangle covered by the glyph bitmap.
    let sx0 = origin.x + b.left * size_px;
    let sx1 = origin.x + b.right * size_px;
    let sy0 = origin.y - b.top * size_px;
    let sy1 = origin.y - b.bottom * size_px;
    let range = entry.screen_px_range(size_px);
    let x_start = (sx0.floor() as i32).max(0);
    let x_end = (sx1.ceil() as i32).min(fb.width() as i32);
    let y_start = (sy0.floor() as i32).max(0);
    let y_end = (sy1.ceil() as i32).min(fb.height() as i32);
    for y in y_start..y_end {
        for x in x_start..x_end {
            // Pixel centre back into plane space (flipping y).
            let plane = Vec2::new(
                (x as f32 + 0.5 - origin.x) / size_px,
                (origin.y - (y as f32 + 0.5)) / size_px,
            );
            let uv = Vec2::new(
                (plane.x - b.left) / b.width().max(1e-9),
                (b.top - plane.y) / b.height().max(1e-9),
            );
            let Some(texel) = atlas.sample_glyph(key, uv) else {
                continue;
            };
            blend_pixel(fb, x, y, color, mtsdf_coverage(texel, range));
        }
    }
    Some(entry.advance * size_px)
}

/// Draw a line (or several, split on `'\n'`) of text. `origin` is
/// the baseline start in screen space; each line drops by
/// `1.25 * size_px`. Unknown glyphs are skipped with a half-unit
/// advance; a space advances by its atlas entry when present, else
/// by `0.4 * size_px`. Returns the width in screen pixels of the
/// widest line drawn.
pub fn draw_text(
    fb: &mut Framebuffer,
    atlas: &TextAtlas,
    text: &str,
    origin: Vec2,
    size_px: f32,
    color: Color,
) -> f32 {
    let mut widest = 0.0_f32;
    let mut cursor = origin;
    let line_start_x = origin.x;
    for ch in text.chars() {
        if ch == '\n' {
            widest = widest.max(cursor.x - line_start_x);
            cursor.x = line_start_x;
            cursor.y += 1.25 * size_px;
            continue;
        }
        if ch == ' ' {
            let adv = atlas
                .glyph(' ')
                .map_or(0.4 * size_px, |e| e.advance * size_px);
            cursor.x += adv;
            continue;
        }
        match draw_glyph(fb, atlas, ch, cursor, size_px, color) {
            Some(adv) => cursor.x += adv,
            None => cursor.x += 0.5 * size_px,
        }
    }
    widest.max(cursor.x - line_start_x)
}

/// The advance width of `text` at `size_px`, in screen pixels
/// (the widest line for multi-line text).
#[must_use]
pub fn measure_text(atlas: &TextAtlas, text: &str, size_px: f32) -> f32 {
    let mut widest = 0.0_f32;
    let mut line = 0.0_f32;
    for ch in text.chars() {
        if ch == '\n' {
            widest = widest.max(line);
            line = 0.0;
        } else if ch == ' ' {
            line += atlas
                .glyph(' ')
                .map_or(0.4 * size_px, |e| e.advance * size_px);
        } else {
            line += atlas
                .glyph(ch)
                .map_or(0.5 * size_px, |e| e.advance * size_px);
        }
    }
    widest.max(line)
}

// ── Demo glyphs ──────────────────────────────────────────────────────────────
//
// A tiny hand-built vector font (cap-height 1, y-up, baseline 0) in
// the same spirit as the analytic shapes elsewhere in the crate: it
// exists so the atlas, the shader and the demo have real outlines to
// work on without loading a font file. Coverage: A D E F I L M O S T
// and space — enough for the demo strings and the tests.

fn glyph_from_polys(contours: &[&[Vec2]], advance: f32, key: char) -> GlyphSource {
    GlyphSource {
        key,
        shape: Shape {
            contours: contours
                .iter()
                .map(|pts| Contour::polygon(pts, EdgeColor::White))
                .collect(),
        },
        advance,
    }
}

const fn v(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

/// The built-in demo glyph for `key`, if one exists.
#[must_use]
pub fn demo_glyph(key: char) -> Option<GlyphSource> {
    let src = match key {
        'A' => glyph_from_polys(
            &[
                &[
                    v(0.00, 0.00),
                    v(0.30, 1.00),
                    v(0.38, 1.00),
                    v(0.68, 0.00),
                    v(0.54, 0.00),
                    v(0.49, 0.18),
                    v(0.19, 0.18),
                    v(0.14, 0.00),
                ],
                &[v(0.34, 0.32), v(0.42, 0.60), v(0.50, 0.32)],
            ],
            0.68,
            'A',
        ),
        'D' => glyph_from_polys(
            &[
                &[
                    v(0.00, 0.00),
                    v(0.00, 1.00),
                    v(0.36, 1.00),
                    v(0.62, 0.82),
                    v(0.68, 0.50),
                    v(0.62, 0.18),
                    v(0.36, 0.00),
                ],
                &[
                    v(0.16, 0.18),
                    v(0.16, 0.82),
                    v(0.34, 0.82),
                    v(0.48, 0.66),
                    v(0.50, 0.50),
                    v(0.48, 0.34),
                    v(0.34, 0.18),
                ],
            ],
            0.68,
            'D',
        ),
        'E' => glyph_from_polys(
            &[&[
                v(0.00, 0.00),
                v(0.00, 1.00),
                v(0.60, 1.00),
                v(0.60, 0.84),
                v(0.16, 0.84),
                v(0.16, 0.58),
                v(0.52, 0.58),
                v(0.52, 0.42),
                v(0.16, 0.42),
                v(0.16, 0.16),
                v(0.60, 0.16),
                v(0.60, 0.00),
            ]],
            0.60,
            'E',
        ),
        'F' => glyph_from_polys(
            &[&[
                v(0.00, 0.00),
                v(0.00, 1.00),
                v(0.60, 1.00),
                v(0.60, 0.84),
                v(0.16, 0.84),
                v(0.16, 0.58),
                v(0.52, 0.58),
                v(0.52, 0.42),
                v(0.16, 0.42),
                v(0.16, 0.00),
            ]],
            0.58,
            'F',
        ),
        'I' => glyph_from_polys(
            &[&[
                v(0.00, 0.00),
                v(0.00, 0.14),
                v(0.10, 0.14),
                v(0.10, 0.86),
                v(0.00, 0.86),
                v(0.00, 1.00),
                v(0.36, 1.00),
                v(0.36, 0.86),
                v(0.26, 0.86),
                v(0.26, 0.14),
                v(0.36, 0.14),
                v(0.36, 0.00),
            ]],
            0.36,
            'I',
        ),
        'L' => glyph_from_polys(
            &[&[
                v(0.00, 0.00),
                v(0.00, 1.00),
                v(0.16, 1.00),
                v(0.16, 0.16),
                v(0.58, 0.16),
                v(0.58, 0.00),
            ]],
            0.58,
            'L',
        ),
        'M' => glyph_from_polys(
            &[&[
                v(0.00, 0.00),
                v(0.00, 1.00),
                v(0.18, 1.00),
                v(0.36, 0.55),
                v(0.54, 1.00),
                v(0.72, 1.00),
                v(0.72, 0.00),
                v(0.56, 0.00),
                v(0.56, 0.68),
                v(0.36, 0.28),
                v(0.16, 0.68),
                v(0.16, 0.00),
            ]],
            0.72,
            'M',
        ),
        'O' => {
            let mut outer = Vec::with_capacity(20);
            let mut inner = Vec::with_capacity(20);
            for i in 0..20 {
                let a = std::f32::consts::TAU * i as f32 / 20.0;
                outer.push(v(0.36 + 0.36 * a.cos(), 0.50 + 0.50 * a.sin()));
                inner.push(v(0.36 + 0.20 * a.cos(), 0.50 + 0.34 * a.sin()));
            }
            glyph_from_polys(&[&outer, &inner], 0.72, 'O')
        }
        'S' => glyph_from_polys(
            &[&[
                v(0.08, 0.74),
                v(0.14, 0.90),
                v(0.28, 0.99),
                v(0.46, 1.00),
                v(0.60, 0.93),
                v(0.65, 0.80),
                v(0.51, 0.72),
                v(0.45, 0.83),
                v(0.33, 0.86),
                v(0.23, 0.81),
                v(0.22, 0.72),
                v(0.28, 0.64),
                v(0.52, 0.50),
                v(0.62, 0.40),
                v(0.63, 0.24),
                v(0.55, 0.09),
                v(0.40, 0.01),
                v(0.22, 0.00),
                v(0.08, 0.06),
                v(0.03, 0.18),
                v(0.16, 0.26),
                v(0.22, 0.15),
                v(0.34, 0.12),
                v(0.46, 0.15),
                v(0.48, 0.24),
                v(0.42, 0.32),
                v(0.16, 0.46),
                v(0.07, 0.58),
            ]],
            0.66,
            'S',
        ),
        'T' => glyph_from_polys(
            &[&[
                v(0.00, 0.84),
                v(0.00, 1.00),
                v(0.64, 1.00),
                v(0.64, 0.84),
                v(0.40, 0.84),
                v(0.40, 0.00),
                v(0.24, 0.00),
                v(0.24, 0.84),
            ]],
            0.64,
            'T',
        ),
        ' ' => GlyphSource {
            key: ' ',
            shape: Shape::default(),
            advance: 0.40,
        },
        _ => return None,
    };
    Some(src)
}

/// All built-in demo glyphs: `A D E F I L M O S T` and space.
#[must_use]
pub fn demo_glyphs() -> Vec<GlyphSource> {
    "ADEFILMOST ".chars().filter_map(demo_glyph).collect()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn square_shape(color: EdgeColor) -> Shape {
        Shape {
            contours: vec![Contour::polygon(
                &[
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(0.0, 1.0),
                ],
                color,
            )],
        }
    }

    fn demo_atlas() -> TextAtlas {
        TextAtlas::build(&demo_glyphs(), GeneratorConfig::default(), 256)
    }

    // ── Spec: median and encoding ────────────────────────────────────

    #[test]
    fn median3_picks_the_middle_for_every_permutation() {
        for (a, b, c) in [
            (0.2, 0.5, 0.9),
            (0.9, 0.2, 0.5),
            (0.5, 0.9, 0.2),
            (0.0, 0.0, 1.0),
            (1.0, 0.0, 1.0),
            (0.3, 0.3, 0.3),
        ] {
            let m = median3(a, b, c);
            let mut sorted = [a, b, c];
            sorted.sort_by(|x, y| x.partial_cmp(y).unwrap());
            assert!((m - sorted[1]).abs() < 1e-7, "median3({a},{b},{c})={m}");
        }
    }

    #[test]
    fn encoding_is_half_on_the_edge_and_monotonic() {
        let mut prev = f32::INFINITY;
        for i in -20..=20 {
            let e = encode_distance(i as f32 * 0.5, 4.0);
            assert!((0.0..=1.0).contains(&e));
            assert!(e <= prev + 1e-7, "encoding rose at step {i}");
            prev = e;
        }
        assert_eq!(encode_distance(0.0, 0.0), 0.5); // degenerate range: step
        assert_eq!(encode_distance(-1.0, 0.0), 1.0);
        assert_eq!(encode_distance(1.0, 0.0), 0.0);
    }

    #[test]
    fn coverage_is_monotonic_in_the_reconstructed_distance() {
        let mut prev = -1.0_f32;
        for i in 0..=20 {
            let v = i as f32 / 20.0;
            let c = mtsdf_coverage([v, v, v, v], 4.0);
            assert!((0.0..=1.0).contains(&c));
            assert!(c >= prev - 1e-7, "coverage fell at {v}");
            prev = c;
        }
    }

    // ── Spec: segment distances ──────────────────────────────────────

    #[test]
    fn line_distance_is_exact() {
        let seg = EdgeSegment::Line {
            from: Vec2::new(0.0, 0.0),
            to: Vec2::new(2.0, 0.0),
        };
        assert!((seg.distance(Vec2::new(1.0, 1.0)) - 1.0).abs() < 1e-6);
        assert!((seg.distance(Vec2::new(3.0, 0.0)) - 1.0).abs() < 1e-6); // past the end
        assert!(seg.distance(Vec2::new(0.5, 0.0)).abs() < 1e-6); // on it
    }

    #[test]
    fn quadratic_distance_hits_the_apex() {
        let seg = EdgeSegment::Quadratic {
            from: Vec2::new(0.0, 0.0),
            ctrl: Vec2::new(0.5, 1.0),
            to: Vec2::new(1.0, 0.0),
        };
        // Apex at t=0.5 is (0.5, 0.5).
        assert!(seg.distance(Vec2::new(0.5, 0.5)) < 1e-4);
        assert!((seg.distance(Vec2::new(0.5, 1.0)) - 0.5).abs() < 1e-3);
        assert!(seg.distance(Vec2::new(0.0, 0.0)) < 1e-6);
    }

    #[test]
    fn cubic_distance_finds_curve_and_endpoints() {
        let seg = EdgeSegment::Cubic {
            from: Vec2::new(0.0, 0.0),
            ctrl1: Vec2::new(0.0, 1.0),
            ctrl2: Vec2::new(1.0, 1.0),
            to: Vec2::new(1.0, 0.0),
        };
        let mid = seg.point(0.5);
        assert!(seg.distance(mid) < 1e-4, "on-curve point must be ~0");
        assert!(seg.distance(Vec2::new(1.0, 0.0)) < 1e-6);
        assert!(seg.distance(Vec2::new(0.5, 2.0)) > 1.0);
    }

    // ── Spec: inside test and signed distances ───────────────────────

    #[test]
    fn inside_test_respects_holes() {
        let a = demo_glyph('A').unwrap().shape;
        // The counter (hole) centroid is outside the ink.
        assert!(!a.is_inside(Vec2::new(0.42, 0.41)));
        // The crossbar below the hole is ink.
        assert!(a.is_inside(Vec2::new(0.42, 0.20)));
        // Far away is outside.
        assert!(!a.is_inside(Vec2::new(2.0, 0.5)));
    }

    #[test]
    fn signed_distances_follow_the_crate_convention() {
        let sq = square_shape(EdgeColor::White);
        assert!(sq.true_distance(Vec2::new(0.5, 0.5)) < 0.0);
        assert!(sq.true_distance(Vec2::new(1.5, 0.5)) > 0.0);
        assert!(sq.true_distance(Vec2::new(1.0, 0.5)).abs() < 1e-6);
        // A white shape measures the same in every channel.
        for ch in 0..3 {
            assert!(
                (sq.channel_distance(Vec2::new(0.5, 0.25), ch)
                    - sq.true_distance(Vec2::new(0.5, 0.25)))
                .abs()
                    < 1e-6
            );
        }
    }

    // ── Spec: edge colouring ─────────────────────────────────────────

    #[test]
    fn square_corners_separate_every_adjacent_pair() {
        let sq = square_shape(EdgeColor::White).colored(DEFAULT_CORNER_THRESHOLD_DEG);
        let edges = &sq.contours[0].edges;
        assert_eq!(edges.len(), 4);
        for i in 0..4 {
            let a = edges[i].color;
            let b = edges[(i + 1) % 4].color;
            assert_ne!(a, b, "edges {i} and {} share colour {a:?}", (i + 1) % 4);
            // And they must differ in at least one channel mask bit.
            assert_ne!(a.channel_mask() ^ b.channel_mask(), 0);
        }
    }

    #[test]
    fn smooth_contour_keeps_one_colour() {
        // A 20-gon "circle": every vertex is a smooth join (18 degrees
        // of turn), so the colour never changes.
        let mut pts = Vec::new();
        for i in 0..20 {
            let a = std::f32::consts::TAU * i as f32 / 20.0;
            pts.push(Vec2::new(0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin()));
        }
        let shape = Shape {
            contours: vec![Contour::polygon(&pts, EdgeColor::White)],
        }
        .colored(DEFAULT_CORNER_THRESHOLD_DEG);
        let first = shape.contours[0].edges[0].color;
        assert!(shape.contours[0].edges.iter().all(|e| e.color == first));
    }

    #[test]
    fn channels_measure_different_edges_at_a_corner() {
        // The mechanism of MSDF: just outside the square's right edge
        // near the top corner, the R channel is fed only by the bottom
        // and top edges, so it reports a *larger* distance than the
        // true distance (which is to the right edge), while G and B
        // still see the right edge. The median recovers the truth.
        let sq = square_shape(EdgeColor::White).colored(DEFAULT_CORNER_THRESHOLD_DEG);
        let p = Vec2::new(1.05, 0.9);
        let true_d = sq.true_distance(p);
        assert!((true_d - 0.05).abs() < 1e-5, "true={true_d}");
        let r = sq.channel_distance(p, 0);
        let g = sq.channel_distance(p, 1);
        let b = sq.channel_distance(p, 2);
        assert!(
            r > true_d + 0.03,
            "R should ignore the right edge: r={r} true={true_d}"
        );
        assert!((g - true_d).abs() < 1e-5, "g={g}");
        assert!((b - true_d).abs() < 1e-5, "b={b}");
        assert!((median3(r, g, b) - true_d).abs() < 1e-5);
    }

    // ── Spec: glyph generation ───────────────────────────────────────

    #[test]
    fn generated_bitmap_has_the_edge_at_half_and_saturated_interior() {
        let sq = square_shape(EdgeColor::White);
        let bounds = Bounds {
            left: -0.5,
            bottom: -0.5,
            right: 1.5,
            top: 1.5,
        };
        let cfg = GeneratorConfig {
            px_range: 2.0,
            pixels_per_unit: 4.0,
            kind: AtlasKind::Mtsdf,
        };
        let bmp = generate_mtsdf(&sq, 8, 8, bounds, cfg);
        // Centre texel (plane ~(0.5, 0.5)): deep inside, all channels 1.
        let centre = bmp.texel(4, 4);
        assert!(
            median3(centre[0], centre[1], centre[2]) > 0.99,
            "centre={centre:?}"
        );
        assert!(centre[3] > 0.99, "alpha is the true distance: {centre:?}");
        // Corner texel (plane ~(-0.375, -0.375)): outside, median < 0.5.
        let corner = bmp.texel(0, 7);
        assert!(
            median3(corner[0], corner[1], corner[2]) < 0.5,
            "corner={corner:?}"
        );
        // A texel just inside the left edge vs just outside it.
        let inside = bmp.texel(2, 4); // plane x = 0.125
        let outside = bmp.texel(1, 4); // plane x = -0.125
        assert!(median3(inside[0], inside[1], inside[2]) > 0.5);
        assert!(median3(outside[0], outside[1], outside[2]) < 0.5);
    }

    #[test]
    fn msdf_kind_leaves_alpha_opaque() {
        let sq = square_shape(EdgeColor::White);
        let bounds = Bounds {
            left: 0.0,
            bottom: 0.0,
            right: 1.0,
            top: 1.0,
        };
        let bmp = generate_mtsdf(
            &sq,
            4,
            4,
            bounds,
            GeneratorConfig {
                kind: AtlasKind::Msdf,
                ..GeneratorConfig::default()
            },
        );
        assert!(bmp.rgba.chunks_exact(4).all(|px| px[3] == 255));
    }

    #[test]
    fn bilinear_sampling_hits_texel_centres_and_midpoints() {
        let bmp = GlyphBitmap {
            width: 2,
            height: 1,
            rgba: vec![0, 0, 0, 255, 255, 255, 255, 255],
            plane_bounds: Bounds {
                left: 0.0,
                bottom: 0.0,
                right: 1.0,
                top: 1.0,
            },
            px_range: 4.0,
            kind: AtlasKind::Mtsdf,
        };
        assert_eq!(bmp.sample_bilinear(0.0, 0.0)[0], 0.0);
        assert_eq!(bmp.sample_bilinear(1.0, 0.0)[0], 1.0);
        assert!((bmp.sample_bilinear(0.5, 0.0)[0] - 0.5).abs() < 1e-6);
    }

    // ── Spec: atlas packing ──────────────────────────────────────────

    #[test]
    fn atlas_packing_is_deterministic_and_non_overlapping() {
        let a = demo_atlas();
        let b = demo_atlas();
        assert_eq!(a.rgba, b.rgba, "same inputs must pack identically");
        assert_eq!(a.glyphs, b.glyphs);
        let entries: Vec<&GlyphEntry> = a.glyphs.values().collect();
        for e in &entries {
            assert!(e.atlas_x + e.width <= a.width);
            assert!(e.atlas_y + e.height <= a.height);
        }
        for i in 0..entries.len() {
            for j in i + 1..entries.len() {
                let (p, q) = (entries[i], entries[j]);
                if p.width == 0 || q.width == 0 {
                    continue;
                }
                let overlap = p.atlas_x < q.atlas_x + q.width
                    && q.atlas_x < p.atlas_x + p.width
                    && p.atlas_y < q.atlas_y + q.height
                    && q.atlas_y < p.atlas_y + p.height;
                assert!(!overlap, "glyph rects {p:?} and {q:?} overlap");
            }
        }
    }

    #[test]
    fn atlas_keeps_advances_and_the_space_is_empty() {
        let atlas = demo_atlas();
        let space = atlas.glyph(' ').unwrap();
        assert_eq!(space.width, 0);
        assert!((space.advance - 0.40).abs() < 1e-6);
        let m = atlas.glyph('M').unwrap();
        assert!(m.width > 0 && m.height > 0);
        assert!((m.advance - 0.72).abs() < 1e-6);
        assert!(atlas.glyph('Z').is_none());
        assert!(atlas.sample_glyph('Z', Vec2::ZERO).is_none());
        assert!(atlas.sample_glyph(' ', Vec2::ZERO).is_none());
    }

    #[test]
    fn atlas_centre_of_a_filled_glyph_is_inside() {
        let atlas = demo_atlas();
        // The middle of the 'I' stem: uv near the bitmap centre must
        // reconstruct as inside.
        let texel = atlas.sample_glyph('I', Vec2::new(0.5, 0.5)).unwrap();
        assert!(mtsdf_distance(texel) > 0.5, "texel={texel:?}");
        // Far outside the atlas glyph quad the sample clamps to the
        // edge texel, which is outside the ink for a padded glyph.
        let edge = atlas.sample_glyph('I', Vec2::new(0.0, 0.0)).unwrap();
        assert!(mtsdf_distance(edge) < 0.5, "edge={edge:?}");
    }

    // ── Rendering ────────────────────────────────────────────────────

    #[test]
    fn draw_text_paints_interior_exterior_and_antialiased_edges() {
        let atlas = demo_atlas();
        let mut fb = Framebuffer::new(320, 120).unwrap();
        let bg = 0xFF10_1018_u32;
        fb.clear(bg);
        let width = draw_text(
            &mut fb,
            &atlas,
            "MSDF",
            Vec2::new(16.0, 96.0),
            72.0,
            Color::WHITE,
        );
        assert!(width > 150.0, "drawn width={width}");
        // Deep inside the M stem (plane ~(0.05, 0.6) of the first glyph).
        assert_eq!(fb.get_pixel(20, 55), Some(Color::WHITE.to_argb_u32()));
        // Far outside: untouched.
        assert_eq!(fb.get_pixel(300, 10), Some(bg));
        assert_eq!(fb.get_pixel(4, 4), Some(bg));
        // The whole point: some pixels are partial blends.
        let partial = fb
            .as_slice()
            .iter()
            .filter(|&&px| px != bg && px != Color::WHITE.to_argb_u32())
            .count();
        assert!(partial > 20, "only {partial} AA edge pixels");
    }

    #[test]
    fn the_same_atlas_stays_antialiased_at_small_sizes() {
        let atlas = demo_atlas();
        for size in [16.0_f32, 96.0] {
            let mut fb = Framebuffer::new(160, 120).unwrap();
            let bg = 0xFF00_0000_u32;
            fb.clear(bg);
            draw_text(
                &mut fb,
                &atlas,
                "A",
                Vec2::new(8.0, 100.0),
                size,
                Color::WHITE,
            );
            let painted = fb.as_slice().iter().filter(|&&px| px != bg).count();
            let partial = fb
                .as_slice()
                .iter()
                .filter(|&&px| px != bg && px != Color::WHITE.to_argb_u32())
                .count();
            assert!(painted > 10, "size {size}: only {painted} painted pixels");
            assert!(partial > 0, "size {size}: no AA pixels — edges went hard");
        }
    }

    #[test]
    fn measure_text_matches_advances() {
        let atlas = demo_atlas();
        let one = measure_text(&atlas, "I", 100.0);
        let two = measure_text(&atlas, "II", 100.0);
        assert!((two - 2.0 * one).abs() < 1e-3, "one={one} two={two}");
        let spaced = measure_text(&atlas, "I I", 100.0);
        assert!((spaced - (two + 40.0)).abs() < 1e-3, "spaced={spaced}");
        // Unknown glyphs still take room.
        assert!(measure_text(&atlas, "Z", 100.0) > 0.0);
    }

    #[test]
    fn draw_glyph_unknown_and_space() {
        let atlas = demo_atlas();
        let mut fb = Framebuffer::new(64, 64).unwrap();
        assert!(
            draw_glyph(
                &mut fb,
                &atlas,
                'Z',
                Vec2::new(8.0, 56.0),
                32.0,
                Color::WHITE
            )
            .is_none()
        );
        let adv = draw_glyph(
            &mut fb,
            &atlas,
            ' ',
            Vec2::new(8.0, 56.0),
            32.0,
            Color::WHITE,
        );
        assert!((adv.unwrap() - 12.8).abs() < 1e-4);
        assert!(
            fb.as_slice().iter().all(|&px| px == 0xFF00_0000),
            "space must not paint"
        );
    }

    #[test]
    fn demo_font_covers_the_demo_alphabet() {
        for ch in "ADEFILMOST ".chars() {
            let g = demo_glyph(ch).unwrap_or_else(|| panic!("missing demo glyph {ch}"));
            if ch != ' ' {
                assert!(!g.shape.contours.is_empty(), "glyph {ch} has no outline");
                assert!(g.shape.bounds().unwrap().width() > 0.1);
            }
        }
        assert!(demo_glyph('Z').is_none());
    }
}
