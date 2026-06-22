//! Color gradient — a sorted list of color stops for smooth interpolation.
//!
//! [`Gradient`] maps a normalized `[0, 1]` parameter to a [`Color`] by
//! linearly interpolating between the two nearest stops.  Stops are sorted
//! by position on construction, so insertion order does not matter.
//!
//! # Use cases
//!
//! - HUD health/energy bars
//! - Terrain height maps (blue → green → brown → white)
//! - Particle lifetime coloring (bright spawn → dark fade)
//! - Heat-map overlays
//!
//! # Examples
//!
//! ```
//! use abrash_core::gradient::Gradient;
//! use abrash_core::color::Color;
//!
//! let grad = Gradient::new(vec![
//!     (0.0, Color::BLUE),
//!     (0.5, Color::GREEN),
//!     (1.0, Color::RED),
//! ]);
//!
//! let mid = grad.sample(0.5);
//! // Exactly on the green stop
//! assert!((mid.g - 1.0).abs() < 1e-4);
//! ```

use crate::color::Color;

/// A single color stop: position in `[0, 1]` and its associated color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorStop {
    /// The scalar anchor point of this stop, typically along the `[0.0, 1.0]` continuum.
    pub position: f32,
    /// The RGB/RGBA color value at this anchor point.
    pub color: Color,
}

impl ColorStop {
    /// Create a new stop.
    #[must_use]
    pub const fn new(position: f32, color: Color) -> Self {
        Self { position, color }
    }
}

/// A sorted list of color stops that can be sampled at any position in `[0, 1]`.
///
/// Requires at least one stop.  Positions outside `[0, 1]` are clamped to the
/// first or last stop's color.
#[derive(Debug, Clone)]
pub struct Gradient {
    stops: Vec<ColorStop>,
}

impl Gradient {
    /// Construct from `(position, color)` pairs.
    ///
    /// Stops are sorted by position; duplicates at the same position keep the
    /// last-inserted color.
    ///
    /// # Panics
    ///
    /// Panics if `stops` is empty.
    #[must_use]
    pub fn new(stops: impl IntoIterator<Item = (f32, Color)>) -> Self {
        let mut v: Vec<ColorStop> = stops
            .into_iter()
            .map(|(p, c)| ColorStop::new(p, c))
            .collect();
        assert!(!v.is_empty(), "Gradient requires at least one stop");
        v.sort_by(|a, b| a.position.total_cmp(&b.position));
        Self { stops: v }
    }

    /// Construct from pre-built [`ColorStop`] slices.
    ///
    /// # Panics
    ///
    /// Panics if `stops` is empty.
    #[must_use]
    pub fn from_stops(stops: impl IntoIterator<Item = ColorStop>) -> Self {
        let mut v: Vec<ColorStop> = stops.into_iter().collect();
        assert!(!v.is_empty(), "Gradient requires at least one stop");
        v.sort_by(|a, b| a.position.total_cmp(&b.position));
        Self { stops: v }
    }

    /// Number of stops.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.stops.len()
    }

    /// Returns `true` if the gradient has no stops (impossible via the public API, but
    /// exposed for completeness).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.stops.is_empty()
    }

    /// View the sorted stops.
    #[must_use]
    pub fn stops(&self) -> &[ColorStop] {
        &self.stops
    }

    /// Sample the gradient at position `t ∈ [0, 1]`.
    ///
    /// - `t ≤ first_stop.position` → first stop's color
    /// - `t ≥ last_stop.position` → last stop's color
    /// - otherwise: linear interpolation between the two enclosing stops
    #[must_use]
    pub fn sample(&self, t: f32) -> Color {
        let stops = &self.stops;

        // Clamp to first/last stop
        if t <= stops[0].position {
            return stops[0].color;
        }
        let last = stops[stops.len() - 1];
        if t >= last.position {
            return last.color;
        }

        // Binary search for the right segment
        let idx = stops.partition_point(|s| s.position <= t);
        // idx is the index of the first stop > t, so the segment is [idx-1, idx]
        let lo = &stops[idx - 1];
        let hi = &stops[idx];

        let range = hi.position - lo.position;
        let local_t = if range > 0.0 {
            (t - lo.position) / range
        } else {
            0.5
        };

        lo.color.lerp(hi.color, local_t)
    }

    /// Sample `n` evenly-spaced points from `t=0` to `t=1`, inclusive.
    ///
    /// Returns `n` colors; `n` must be ≥ 1.
    ///
    /// # Panics
    ///
    /// Panics if `n == 0`.
    ///
    #[must_use]
    pub fn sample_n(&self, n: usize) -> Vec<Color> {
        assert!(n > 0, "sample_n requires n >= 1");
        if n == 1 {
            return vec![self.sample(0.5)];
        }
        (0..n)
            .map(|i| self.sample(i as f32 / (n - 1) as f32))
            .collect()
    }

    /// Return a new gradient with an additional stop inserted.
    #[must_use]
    pub fn with_stop(mut self, position: f32, color: Color) -> Self {
        self.stops.push(ColorStop::new(position, color));
        self.stops.sort_by(|a, b| a.position.total_cmp(&b.position));
        self
    }
}

// ── Built-in palettes ─────────────────────────────────────────────────────────

impl Gradient {
    /// Black → white.
    #[must_use]
    pub fn greyscale() -> Self {
        Self::new([(0.0, Color::BLACK), (1.0, Color::WHITE)])
    }

    /// Hot: black → red → yellow → white (thermal / heat map).
    #[must_use]
    pub fn heat() -> Self {
        Self::new([
            (0.0, Color::BLACK),
            (0.33, Color::RED),
            (0.67, Color::YELLOW),
            (1.0, Color::WHITE),
        ])
    }

    /// Health bar: red → yellow → green.
    #[must_use]
    pub fn health() -> Self {
        Self::new([(0.0, Color::RED), (0.5, Color::YELLOW), (1.0, Color::GREEN)])
    }

    /// Terrain: deep blue → green → brown → white.
    #[must_use]
    pub fn terrain() -> Self {
        Self::new([
            (0.0, Color::new(0.0, 0.0, 0.5, 1.0)),    // deep blue (water)
            (0.3, Color::new(0.13, 0.55, 0.13, 1.0)), // forest green
            (0.6, Color::new(0.55, 0.40, 0.23, 1.0)), // earthy brown
            (0.85, Color::new(0.7, 0.7, 0.7, 1.0)),   // grey rock
            (1.0, Color::WHITE),                      // snow
        ])
    }

    /// Plasma: dark violet → magenta → orange → yellow (like matplotlib "plasma").
    #[must_use]
    pub fn plasma() -> Self {
        Self::new([
            (0.0, Color::new(0.05, 0.03, 0.53, 1.0)),
            (0.25, Color::new(0.49, 0.01, 0.66, 1.0)),
            (0.5, Color::new(0.80, 0.07, 0.44, 1.0)),
            (0.75, Color::new(0.97, 0.43, 0.06, 1.0)),
            (1.0, Color::new(0.94, 0.98, 0.13, 1.0)),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_stop_always_returns_that_color() {
        let g = Gradient::new([(0.5, Color::RED)]);
        let c = g.sample(0.0);
        assert!((c.r - 1.0).abs() < 1e-4);
        assert!((c.g).abs() < 1e-4);
    }

    #[test]
    fn endpoints_return_stop_colors() {
        let g = Gradient::new([(0.0, Color::RED), (1.0, Color::BLUE)]);
        let lo = g.sample(0.0);
        let hi = g.sample(1.0);
        assert!((lo.r - 1.0).abs() < 1e-4 && lo.b.abs() < 1e-4);
        assert!((hi.b - 1.0).abs() < 1e-4 && hi.r.abs() < 1e-4);
    }

    #[test]
    fn midpoint_interpolation() {
        let g = Gradient::new([(0.0, Color::BLACK), (1.0, Color::WHITE)]);
        let mid = g.sample(0.5);
        assert!((mid.r - 0.5).abs() < 1e-4);
        assert!((mid.g - 0.5).abs() < 1e-4);
        assert!((mid.b - 0.5).abs() < 1e-4);
    }

    #[test]
    fn out_of_range_clamped() {
        let g = Gradient::new([(0.0, Color::RED), (1.0, Color::BLUE)]);
        let below = g.sample(-1.0);
        let above = g.sample(2.0);
        assert!((below.r - 1.0).abs() < 1e-4);
        assert!((above.b - 1.0).abs() < 1e-4);
    }

    #[test]
    fn stops_sorted_on_construct() {
        // Insert in wrong order — should still work
        let g = Gradient::new([(1.0, Color::BLUE), (0.0, Color::RED)]);
        let lo = g.sample(0.0);
        assert!((lo.r - 1.0).abs() < 1e-4);
    }

    #[test]
    fn three_stop_midpoints() {
        let g = Gradient::new([(0.0, Color::RED), (0.5, Color::GREEN), (1.0, Color::BLUE)]);
        // Exactly on the middle stop
        let mid = g.sample(0.5);
        assert!((mid.g - 1.0).abs() < 1e-4, "g={}", mid.g);
        assert!(mid.r.abs() < 1e-4, "r={}", mid.r);
        // Quarter-way: halfway between red and green
        let q = g.sample(0.25);
        assert!((q.r - 0.5).abs() < 1e-4);
        assert!((q.g - 0.5).abs() < 1e-4);
    }

    #[test]
    fn sample_n_count() {
        let g = Gradient::greyscale();
        let samples = g.sample_n(5);
        assert_eq!(samples.len(), 5);
        // First = black, last = white
        assert!(samples[0].r.abs() < 1e-4);
        assert!((samples[4].r - 1.0).abs() < 1e-4);
    }

    #[test]
    fn with_stop_inserts_correctly() {
        let g =
            Gradient::new([(0.0, Color::BLACK), (1.0, Color::WHITE)]).with_stop(0.5, Color::RED);
        assert_eq!(g.len(), 3);
        let mid = g.sample(0.5);
        assert!((mid.r - 1.0).abs() < 1e-4);
    }

    #[test]
    fn built_in_gradients_sample_cleanly() {
        for g in [
            Gradient::greyscale(),
            Gradient::heat(),
            Gradient::health(),
            Gradient::terrain(),
            Gradient::plasma(),
        ] {
            // Just verify they don't panic and return sane alpha
            let mid = g.sample(0.5);
            assert!((mid.a - 1.0).abs() < 1e-4);
        }
    }
}
