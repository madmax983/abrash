#![allow(clippy::float_cmp, clippy::imprecise_flops)]
//! Two-dimensional Radiance Cascades (Sannikov-style) global
//! illumination for a grid scene — a CPU reference solver.
//!
//! Radiance Cascades split the light field into a stack of cascades.
//! Each cascade samples the scene with probes on a coarser grid, with
//! more ray directions, and with rays that cover a longer distance
//! interval. Merging the cascades top-down gives every fine probe the
//! light that arrives from near and far geometry without tracing long
//! rays at full resolution. This module is the CPU reference form of
//! that structure: the same probe layout, interval split, and merge
//! that a GPU port would use, evaluated with exact grid distances.
//!
//! # Scene model
//!
//! The scene is a `width` by `height` grid of [`RcCell`]s. A cell is
//! empty, an occluder (solid, emits black), or an emitter (solid,
//! emits an RGB radiance). Cell `(x, y)` covers the square
//! `[x, x + 1)` by `[y, y + 1)`; its centre is `(x + 0.5, y + 0.5)`.
//! Rays that leave the grid meet the environment, whose radiance is
//! part of [`RcConfig`].
//!
//! # Cascade layout (the rules this solver implements)
//!
//! For cascade level `c`, counting from `0` (the finest):
//!
//! * Probe spacing is `base_spacing << c`. Probe `(i, j)` sits at the
//!   centre of cell `(i * spacing, j * spacing)`, so cascade 0 with
//!   the default spacing of 1 has one probe per pixel.
//! * The direction count is `4 << c`. Direction `j` points at angle
//!   `(j + 0.5) * 2 * PI / count` — the half-step offset at every
//!   level. The two children of direction `j` in the next cascade,
//!   `2 * j` and `2 * j + 1`, then bracket the parent angle
//!   symmetrically, at plus and minus one quarter of the parent step.
//! * Ray intervals partition distance without gaps or overlap. The
//!   interval length of level `c` is `base_len * 4^c` (the classic
//!   Radiance Cascades growth: length grows by four while spacing
//!   only doubles), so `t_start(c) = base_len * (4^c - 1) / 3`,
//!   `t_end(c) = t_start(c) + base_len * 4^c`, and
//!   `t_start(c + 1) == t_end(c)` exactly.
//! * The default cascade count is the smallest count whose total
//!   reach `base_len * (4^N - 1) / 3` covers the scene diagonal.
//!
//! # Tracing and merging
//!
//! Each ray sphere-traces its own interval over the exact signed
//! distance field of the grid (see [`RcScene::signed_distance`]): it
//! advances by the distance to the nearest solid cell and reports a
//! hit when it comes within half a cell of a solid cell or enters
//! one. A hit returns that cell's emission and blocks all light
//! behind it (transmittance zero). A ray with no hit in its interval
//! takes its far radiance from the next cascade up: the upper
//! cascade is bilinearly interpolated at the ray's interval-end
//! position, for the two child directions of the ray, and the two
//! values are averaged. The highest cascade uses the configured
//! environment radiance as its far field.
//!
//! The result in [`RadianceCascades::output`] is, per cascade-0
//! probe, the average radiance over the cascade-0 directions. With
//! the default spacing of 1 that is one value per scene pixel. A
//! probe inside an emitter cell sees that cell's emission; a probe
//! inside an occluder sees black.
//!
//! # Invariants (the spec these functions are tested against)
//!
//! 1. Intervals are contiguous: `t_start(0) == 0` and
//!    `t_start(c + 1) == t_end(c)` for every level.
//! 2. Direction counts double per level, probe spacing doubles per
//!    level, and the children of a ray bracket its angle.
//! 3. An empty scene under a uniform environment returns that
//!    environment radiance at every pixel, exactly.
//! 4. Every output channel is a hit emission or a convex average of
//!    emissions and the environment, so it stays inside
//!    `[0, max(environment, max emission)]` and stays finite.
//! 5. Invalid input (zero dimensions, a wrong cell count, zero
//!    cascades, non-finite or negative radiance) is an error, never
//!    a panic.
//!
//! # Cost and honest limitations
//!
//! The distance field is computed by brute force: each query scans
//! every solid cell, so a query costs `O(solid cells)` and building
//! no acceleration structure costs `O(width * height)` once. That is
//! a scene-build / authoring-time choice for a reference solver;
//! grid sizes in tests and the demo stay small. Sphere tracing with
//! a half-cell hit threshold is exact for axis-aligned rays and for
//! cell entry; a ray that only grazes a corner inside the half-cell
//! band is reported as a hit on the nearest cell, which is the
//! documented threshold rule, not an exact visibility query.
//!
//! # Examples
//!
//! ```
//! use abrash_core::radiance_cascades::{RadianceCascades, RcConfig, RcScene};
//!
//! // An empty room under a uniform environment takes the
//! // environment radiance at every pixel, exactly.
//! let scene = RcScene::empty(4, 3).unwrap();
//! let mut config = RcConfig::default();
//! config.environment = [0.25, 0.5, 0.75];
//! let solver = RadianceCascades::new(scene, config).unwrap();
//! assert!(solver.output().iter().all(|&px| px == [0.25, 0.5, 0.75]));
//! ```

use std::error::Error;
use std::f32::consts::TAU;
use std::fmt;

use crate::math::Vec2;

/// Direction count of cascade 0. Each higher cascade doubles it.
pub const BASE_DIRECTION_COUNT: usize = 4;

/// Growth factor of the ray interval length per cascade level.
pub const INTERVAL_GROWTH: f32 = 4.0;

/// A ray reports a hit when it comes this close to a solid cell.
pub const HIT_DISTANCE: f32 = 0.5;

/// Largest cascade count a configuration may request. This bounds
/// probe spacing, direction counts, and total work.
pub const MAX_CASCADES: usize = 16;

/// Smallest forward step a sphere trace may take. This keeps a ray
/// that runs parallel and close to a wall moving forward.
const MIN_TRACE_STEP: f32 = 0.05;

/// Upper bound on sphere-trace steps for one ray interval.
const MAX_TRACE_STEPS: usize = 4096;

/// Errors returned when a scene or a configuration is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RcError {
    /// The scene width is zero.
    ZeroWidth,
    /// The scene height is zero.
    ZeroHeight,
    /// The cell slice length does not match `width * height`.
    CellCountMismatch {
        /// The length the dimensions require.
        expected: usize,
        /// The length the caller supplied.
        actual: usize,
    },
    /// The dimensions overflow the cell count or the work bound.
    SceneTooLarge,
    /// The base probe spacing is zero.
    ZeroBaseSpacing,
    /// The base interval length is not finite or not positive.
    InvalidBaseIntervalLength,
    /// The requested cascade count is zero.
    ZeroCascadeCount,
    /// The requested cascade count exceeds [`MAX_CASCADES`].
    TooManyCascades {
        /// The count the caller requested.
        requested: usize,
    },
    /// A radiance value (emission or environment) is not finite.
    NonFiniteRadiance,
    /// A radiance value (emission or environment) is negative.
    NegativeRadiance,
    /// A cell coordinate lies outside the scene.
    OutOfBounds {
        /// The x coordinate that was requested.
        x: usize,
        /// The y coordinate that was requested.
        y: usize,
    },
}

impl fmt::Display for RcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWidth => write!(f, "scene width is zero"),
            Self::ZeroHeight => write!(f, "scene height is zero"),
            Self::CellCountMismatch { expected, actual } => {
                write!(f, "cell count mismatch: expected {expected}, got {actual}")
            }
            Self::SceneTooLarge => write!(f, "scene dimensions are too large"),
            Self::ZeroBaseSpacing => write!(f, "base probe spacing is zero"),
            Self::InvalidBaseIntervalLength => {
                write!(f, "base interval length must be finite and positive")
            }
            Self::ZeroCascadeCount => write!(f, "cascade count is zero"),
            Self::TooManyCascades { requested } => {
                write!(f, "cascade count {requested} exceeds {MAX_CASCADES}")
            }
            Self::NonFiniteRadiance => write!(f, "radiance value is not finite"),
            Self::NegativeRadiance => write!(f, "radiance value is negative"),
            Self::OutOfBounds { x, y } => write!(f, "cell ({x}, {y}) is out of bounds"),
        }
    }
}

impl Error for RcError {}

/// Check one radiance value: every channel must be finite and
/// non-negative, so averages of valid values stay valid and bounded.
fn validate_radiance(value: [f32; 3]) -> Result<(), RcError> {
    for channel in value {
        if !channel.is_finite() {
            return Err(RcError::NonFiniteRadiance);
        }
        if channel < 0.0 {
            return Err(RcError::NegativeRadiance);
        }
    }
    Ok(())
}

/// One cell of the scene grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RcCell {
    /// No geometry: rays pass through and the cell emits nothing.
    Empty,
    /// Solid geometry that emits black: it blocks all light behind it.
    Occluder,
    /// Solid geometry that emits the given RGB radiance.
    Emitter([f32; 3]),
}

impl RcCell {
    /// Whether this cell is solid (blocks rays).
    #[must_use]
    pub const fn is_solid(self) -> bool {
        !matches!(self, Self::Empty)
    }

    /// The radiance this cell emits: the emitter value, black for an
    /// occluder, and black for an empty cell (which is never hit).
    #[must_use]
    pub const fn emission(self) -> [f32; 3] {
        match self {
            Self::Emitter(value) => value,
            Self::Empty | Self::Occluder => [0.0, 0.0, 0.0],
        }
    }
}

/// A grid scene for the solver: dimensions plus one cell per pixel,
/// in row-major order.
#[derive(Debug, Clone, PartialEq)]
pub struct RcScene {
    width: usize,
    height: usize,
    cells: Vec<RcCell>,
    /// Indices of the solid cells, built once at construction. The
    /// distance field scans this list, never the full grid.
    solids: Vec<usize>,
}

impl RcScene {
    /// Build a scene from a full row-major cell slice.
    ///
    /// # Errors
    ///
    /// Returns an error for zero dimensions, a cell count that does
    /// not equal `width * height`, or an emitter with a non-finite
    /// or negative radiance.
    pub fn new(width: usize, height: usize, cells: Vec<RcCell>) -> Result<Self, RcError> {
        if width == 0 {
            return Err(RcError::ZeroWidth);
        }
        if height == 0 {
            return Err(RcError::ZeroHeight);
        }
        let expected = width.checked_mul(height).ok_or(RcError::SceneTooLarge)?;
        if cells.len() != expected {
            return Err(RcError::CellCountMismatch {
                expected,
                actual: cells.len(),
            });
        }
        for cell in &cells {
            if let RcCell::Emitter(value) = cell {
                validate_radiance(*value)?;
            }
        }
        let solids = cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.is_solid())
            .map(|(index, _)| index)
            .collect();
        Ok(Self {
            width,
            height,
            cells,
            solids,
        })
    }

    /// Build an all-empty scene.
    ///
    /// # Errors
    ///
    /// Returns an error for zero dimensions.
    pub fn empty(width: usize, height: usize) -> Result<Self, RcError> {
        if width == 0 {
            return Err(RcError::ZeroWidth);
        }
        if height == 0 {
            return Err(RcError::ZeroHeight);
        }
        let count = width.checked_mul(height).ok_or(RcError::SceneTooLarge)?;
        Ok(Self {
            width,
            height,
            cells: vec![RcCell::Empty; count],
            solids: Vec::new(),
        })
    }

    /// The scene width in cells.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// The scene height in cells.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// All cells, in row-major order.
    #[must_use]
    pub fn cells(&self) -> &[RcCell] {
        &self.cells
    }

    /// The cell at `(x, y)`, or `None` outside the scene.
    #[must_use]
    pub fn cell(&self, x: usize, y: usize) -> Option<RcCell> {
        if x < self.width && y < self.height {
            self.cells.get(y * self.width + x).copied()
        } else {
            None
        }
    }

    /// Replace the cell at `(x, y)`.
    ///
    /// # Errors
    ///
    /// Returns an error when `(x, y)` is outside the scene or the
    /// new cell is an emitter with an invalid radiance.
    pub fn set_cell(&mut self, x: usize, y: usize, cell: RcCell) -> Result<(), RcError> {
        if x >= self.width || y >= self.height {
            return Err(RcError::OutOfBounds { x, y });
        }
        if let RcCell::Emitter(value) = cell {
            validate_radiance(value)?;
        }
        let index = y * self.width + x;
        let was_solid = self.cells[index].is_solid();
        self.cells[index] = cell;
        if cell.is_solid() && !was_solid {
            self.solids.push(index);
        } else if !cell.is_solid() && was_solid {
            self.solids.retain(|&solid| solid != index);
        }
        Ok(())
    }

    /// The cell that contains point `p`, or `None` when `p` lies
    /// outside the grid rectangle.
    fn cell_at(&self, p: Vec2) -> Option<RcCell> {
        if p.x < 0.0 || p.y < 0.0 || p.x >= self.width as f32 || p.y >= self.height as f32 {
            return None;
        }
        let x = p.x.floor() as usize;
        let y = p.y.floor() as usize;
        self.cell(x, y)
    }

    /// Distance from point `p` to the square of solid cell `index`,
    /// plus the depth of `p` inside that square when it is inside.
    /// Returns `(outside_distance, inside_depth)`; exactly one of the
    /// two is non-zero for a given cell, and both are zero on an edge.
    fn cell_distance(&self, index: usize, p: Vec2) -> (f32, f32) {
        let cx = (index % self.width) as f32;
        let cy = (index / self.width) as f32;
        let dx = (cx - p.x).max(p.x - (cx + 1.0)).max(0.0);
        let dy = (cy - p.y).max(p.y - (cy + 1.0)).max(0.0);
        let outside = dx.hypot(dy);
        let depth = if outside == 0.0 {
            (p.x - cx)
                .min(cx + 1.0 - p.x)
                .min(p.y - cy)
                .min(cy + 1.0 - p.y)
        } else {
            0.0
        };
        (outside, depth)
    }

    /// The exact signed distance from `p` to the nearest solid cell.
    ///
    /// The distance is positive outside all solid cells, negative
    /// inside a solid cell (minus the depth to its nearest face,
    /// matching the crate convention of negative inside), and
    /// positive infinity when the scene has no solid cells. The
    /// value is the true Euclidean distance to the nearest solid
    /// square, computed by brute force over the solid list in
    /// `O(solid cells)` per query.
    #[must_use]
    pub fn signed_distance(&self, p: Vec2) -> f32 {
        let mut best_outside = f32::INFINITY;
        let mut inside_depth: Option<f32> = None;
        for &index in &self.solids {
            let (outside, depth) = self.cell_distance(index, p);
            if outside == 0.0 {
                inside_depth = Some(depth);
            } else if outside < best_outside {
                best_outside = outside;
            }
        }
        inside_depth.map_or(best_outside, |depth| -depth)
    }

    /// The outside distance to the nearest solid cell and that
    /// cell's index. The distance is zero when `p` is inside or on
    /// a solid cell. Returns `None` when the scene has no solids.
    fn nearest_solid(&self, p: Vec2) -> Option<(f32, usize)> {
        let mut best: Option<(f32, usize)> = None;
        for &index in &self.solids {
            let (outside, _) = self.cell_distance(index, p);
            if best.is_none_or(|(best_distance, _)| outside < best_distance) {
                best = Some((outside, index));
            }
            if outside == 0.0 {
                break;
            }
        }
        best
    }
}

/// Solver configuration. See the module docs for the layout rules
/// each field feeds.
#[derive(Debug, Clone, PartialEq)]
pub struct RcConfig {
    /// Probe spacing of cascade 0, in cells. The default of 1 puts
    /// one cascade-0 probe on every pixel.
    pub base_spacing: usize,
    /// Ray interval length of cascade 0. Higher levels multiply it
    /// by [`INTERVAL_GROWTH`] per level.
    pub base_interval_length: f32,
    /// Number of cascades, or `None` to derive the smallest count
    /// whose reach covers the scene diagonal.
    pub cascade_count: Option<usize>,
    /// Radiance a ray sees when it leaves the grid, and the far
    /// field of the highest cascade.
    pub environment: [f32; 3],
}

impl Default for RcConfig {
    fn default() -> Self {
        Self {
            base_spacing: 1,
            base_interval_length: 1.0,
            cascade_count: None,
            environment: [0.0, 0.0, 0.0],
        }
    }
}

/// Probe grid dimensions for a scene of `width` by `height` sampled
/// every `spacing` cells: `ceil(width / spacing)` by
/// `ceil(height / spacing)`. A zero spacing yields `(0, 0)`.
#[must_use]
pub const fn probe_grid_dims(width: usize, height: usize, spacing: usize) -> (usize, usize) {
    if spacing == 0 {
        return (0, 0);
    }
    (width.div_ceil(spacing), height.div_ceil(spacing))
}

/// Direction count of cascade `level`: `4 << level`. Returns `None`
/// when the shift would overflow.
#[must_use]
pub fn direction_count(level: usize) -> Option<usize> {
    BASE_DIRECTION_COUNT.checked_shl(u32::try_from(level).ok()?)
}

/// Angle in radians of direction `direction` in cascade `level`,
/// with the half-step offset: `(direction + 0.5) * 2 * PI / count`.
/// An out-of-range level yields `0.0`.
#[must_use]
pub fn direction_angle(level: usize, direction: usize) -> f32 {
    direction_count(level).map_or(0.0, |count| (direction as f32 + 0.5) * TAU / count as f32)
}

/// Unit vector of direction `direction` in cascade `level`.
#[must_use]
pub fn direction_vector(level: usize, direction: usize) -> Vec2 {
    Vec2::from_angle(direction_angle(level, direction))
}

/// The two children of ray `direction` in the next cascade up:
/// `(2 * direction, 2 * direction + 1)`. With the half-step offset
/// their angles bracket the parent angle by one quarter step each.
#[must_use]
pub const fn child_directions(direction: usize) -> (usize, usize) {
    (2 * direction, 2 * direction + 1)
}

/// The `[t_start, t_end)` distance interval of cascade `level`.
///
/// Lengths grow by [`INTERVAL_GROWTH`] per level and intervals are
/// contiguous, so `t_start(level + 1) == t_end(level)`.
#[must_use]
pub fn ray_interval(level: usize, base_interval_length: f32) -> (f32, f32) {
    let growth = INTERVAL_GROWTH.powi(i32::try_from(level).unwrap_or(i32::MAX));
    let t_start = base_interval_length * (growth - 1.0) / (INTERVAL_GROWTH - 1.0);
    (t_start, t_start + base_interval_length * growth)
}

/// The default cascade count for a scene: the smallest count whose
/// total reach `base_len * (4^N - 1) / 3` covers the scene diagonal,
/// at least 1 and at most [`MAX_CASCADES`].
#[must_use]
pub fn default_cascade_count(width: usize, height: usize, base_interval_length: f32) -> usize {
    let diagonal = (width as f32).hypot(height as f32);
    let mut count = 1;
    while count < MAX_CASCADES {
        let (_, reach) = ray_interval(count - 1, base_interval_length);
        if reach >= diagonal {
            break;
        }
        count += 1;
    }
    count
}

/// Sphere-trace one ray interval over the scene's distance field.
///
/// The ray starts at `origin`, follows `direction` (which is
/// normalised internally; `t` values are world distances along the
/// normalised direction), and only the segment `[t_start, t_end)` is
/// tested. The trace advances by the exact distance to the nearest
/// solid cell and reports a hit when it enters a solid cell or comes
/// within [`HIT_DISTANCE`] of one.
///
/// Returns the hit cell's emission (black for an occluder), or
/// `None` when the segment holds no hit: the ray left the grid, the
/// interval ended first, or the direction was zero or non-finite.
#[must_use]
pub fn trace_interval(
    scene: &RcScene,
    origin: Vec2,
    direction: Vec2,
    t_start: f32,
    t_end: f32,
) -> Option<[f32; 3]> {
    let length = direction.length();
    if !length.is_finite() || length <= 0.0 || !t_start.is_finite() || !t_end.is_finite() {
        return None;
    }
    let dir = Vec2::new(direction.x / length, direction.y / length);
    let mut t = t_start.max(0.0);
    if t >= t_end {
        return None;
    }
    for _ in 0..MAX_TRACE_STEPS {
        let p = Vec2::new(origin.x + dir.x * t, origin.y + dir.y * t);
        // A straight ray that leaves the grid rectangle never comes
        // back, and outside the grid there is only the environment.
        let cell = scene.cell_at(p)?;
        if cell.is_solid() {
            return Some(cell.emission());
        }
        let (distance, nearest) = scene.nearest_solid(p)?;
        if distance <= HIT_DISTANCE {
            return Some(scene.cells[nearest].emission());
        }
        // The sphere of radius `distance` around `p` holds no solid
        // cell, so no hit can occur before `t + step` either: when
        // that point already passes the interval end, the segment
        // is a miss.
        let step = (distance - HIT_DISTANCE).max(MIN_TRACE_STEP);
        if t + step >= t_end {
            return None;
        }
        t += step;
    }
    None
}

/// One solved cascade level: its probe grid layout and the merged
/// radiance of every probe and direction.
#[derive(Debug, Clone, PartialEq)]
pub struct CascadeLevel {
    /// Level index: 0 is the finest cascade.
    pub index: usize,
    /// Probe spacing in cells for this level.
    pub spacing: usize,
    /// Number of probes along x.
    pub probe_width: usize,
    /// Number of probes along y.
    pub probe_height: usize,
    /// Number of ray directions per probe.
    pub direction_count: usize,
    /// Start of this level's ray interval (inclusive).
    pub t_start: f32,
    /// End of this level's ray interval (exclusive).
    pub t_end: f32,
    /// Merged radiance, row-major over probes, then directions.
    radiance: Vec<[f32; 3]>,
}

impl CascadeLevel {
    /// World position of probe `(probe_x, probe_y)`: the centre of
    /// the cell at `(probe_x * spacing, probe_y * spacing)`.
    #[must_use]
    pub fn probe_position(&self, probe_x: usize, probe_y: usize) -> Vec2 {
        Vec2::new(
            (probe_x * self.spacing) as f32 + 0.5,
            (probe_y * self.spacing) as f32 + 0.5,
        )
    }

    /// The merged radiance of one probe and direction, or `None`
    /// when the probe or direction is out of range.
    #[must_use]
    pub fn ray_radiance(
        &self,
        probe_x: usize,
        probe_y: usize,
        direction: usize,
    ) -> Option<[f32; 3]> {
        if probe_x >= self.probe_width || probe_y >= self.probe_height {
            return None;
        }
        self.radiance
            .get((probe_y * self.probe_width + probe_x) * self.direction_count + direction)
            .copied()
    }
}

/// Bilinearly sample one direction of a solved cascade at world
/// position `pos`, clamping to the edge probes outside the grid.
/// When all four corner values are equal the result is that value
/// exactly, which keeps uniform fields (an empty scene under a
/// uniform environment) exact through every merge.
fn sample_level(level: &CascadeLevel, pos: Vec2, direction: usize) -> [f32; 3] {
    let u = ((pos.x - 0.5) / level.spacing as f32).clamp(0.0, (level.probe_width - 1) as f32);
    let v = ((pos.y - 0.5) / level.spacing as f32).clamp(0.0, (level.probe_height - 1) as f32);
    let x0 = u.floor() as usize;
    let y0 = v.floor() as usize;
    let x1 = (x0 + 1).min(level.probe_width - 1);
    let y1 = (y0 + 1).min(level.probe_height - 1);
    let fx = u - x0 as f32;
    let fy = v - y0 as f32;
    // Probe indices were built from this level's own dimensions, so
    // the four lookups are in range by construction; a missing value
    // falls back to black rather than panicking.
    let corner = |x: usize, y: usize| level.ray_radiance(x, y, direction).unwrap_or([0.0; 3]);
    let c00 = corner(x0, y0);
    let c10 = corner(x1, y0);
    let c01 = corner(x0, y1);
    let c11 = corner(x1, y1);
    if c00 == c10 && c00 == c01 && c00 == c11 {
        return c00;
    }
    let mut out = [0.0; 3];
    for channel in 0..3 {
        let top = c00[channel] * (1.0 - fx) + c10[channel] * fx;
        let bottom = c01[channel] * (1.0 - fx) + c11[channel] * fx;
        out[channel] = top * (1.0 - fy) + bottom * fy;
    }
    out
}

/// A solved scene: all cascade levels plus the cascade-0 result.
///
/// Build one with [`RadianceCascades::new`]; solving happens there,
/// eagerly, so the accessors never fail.
#[derive(Debug, Clone, PartialEq)]
pub struct RadianceCascades {
    scene: RcScene,
    config: RcConfig,
    levels: Vec<CascadeLevel>,
    output: Vec<[f32; 3]>,
}

impl RadianceCascades {
    /// Validate the configuration and solve the scene.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero base spacing, a non-positive or
    /// non-finite base interval length, a zero or excessive cascade
    /// count, or a non-finite or negative environment radiance.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::radiance_cascades::{RadianceCascades, RcCell, RcConfig, RcScene};
    ///
    /// // One emitter cell lights itself with its own emission.
    /// let mut scene = RcScene::empty(3, 3).unwrap();
    /// scene.set_cell(1, 1, RcCell::Emitter([1.0, 0.5, 0.25])).unwrap();
    /// let solver = RadianceCascades::new(scene, RcConfig::default()).unwrap();
    /// assert_eq!(solver.output_pixel(1, 1), Some([1.0, 0.5, 0.25]));
    /// ```
    pub fn new(scene: RcScene, config: RcConfig) -> Result<Self, RcError> {
        if config.base_spacing == 0 {
            return Err(RcError::ZeroBaseSpacing);
        }
        if !config.base_interval_length.is_finite() || config.base_interval_length <= 0.0 {
            return Err(RcError::InvalidBaseIntervalLength);
        }
        validate_radiance(config.environment)?;
        let cascade_count = match config.cascade_count {
            Some(0) => return Err(RcError::ZeroCascadeCount),
            Some(count) if count > MAX_CASCADES => {
                return Err(RcError::TooManyCascades { requested: count });
            }
            Some(count) => count,
            None => {
                default_cascade_count(scene.width(), scene.height(), config.base_interval_length)
            }
        };

        // Solve from the highest cascade down: each level reads only
        // the finished level above it.
        let mut solved: Vec<CascadeLevel> = Vec::with_capacity(cascade_count);
        for index in (0..cascade_count).rev() {
            let level = Self::solve_level(&scene, &config, index, solved.last())?;
            solved.push(level);
        }
        solved.reverse();

        // The output is the cascade-0 average over directions.
        let finest = &solved[0];
        let mut output = vec![[0.0; 3]; finest.probe_width * finest.probe_height];
        for probe_y in 0..finest.probe_height {
            for probe_x in 0..finest.probe_width {
                let mut sum = [0.0; 3];
                for direction in 0..finest.direction_count {
                    if let Some(value) = finest.ray_radiance(probe_x, probe_y, direction) {
                        for channel in 0..3 {
                            sum[channel] += value[channel];
                        }
                    }
                }
                let count = finest.direction_count as f32;
                let target = &mut output[probe_y * finest.probe_width + probe_x];
                for channel in 0..3 {
                    target[channel] = sum[channel] / count;
                }
            }
        }

        Ok(Self {
            scene,
            config,
            levels: solved,
            output,
        })
    }

    /// Solve one cascade level, reading the finished level above it
    /// (`upper` is `None` for the highest cascade).
    fn solve_level(
        scene: &RcScene,
        config: &RcConfig,
        index: usize,
        upper: Option<&CascadeLevel>,
    ) -> Result<CascadeLevel, RcError> {
        let spacing = config
            .base_spacing
            .checked_mul(1usize << index)
            .ok_or(RcError::SceneTooLarge)?;
        let directions = direction_count(index).ok_or(RcError::SceneTooLarge)?;
        let (probe_width, probe_height) = probe_grid_dims(scene.width(), scene.height(), spacing);
        let (t_start, t_end) = ray_interval(index, config.base_interval_length);
        let total = probe_width
            .checked_mul(probe_height)
            .and_then(|probes| probes.checked_mul(directions))
            .ok_or(RcError::SceneTooLarge)?;
        let mut level = CascadeLevel {
            index,
            spacing,
            probe_width,
            probe_height,
            direction_count: directions,
            t_start,
            t_end,
            radiance: vec![[0.0; 3]; total],
        };

        for probe_y in 0..probe_height {
            for probe_x in 0..probe_width {
                let pos = level.probe_position(probe_x, probe_y);
                let base = (probe_y * probe_width + probe_x) * directions;
                // A probe inside a solid cell sees that cell on every
                // ray: its own emission, or black for an occluder.
                if let Some(cell) = scene.cell_at(pos) {
                    if cell.is_solid() {
                        for slot in &mut level.radiance[base..base + directions] {
                            *slot = cell.emission();
                        }
                        continue;
                    }
                }
                for direction in 0..directions {
                    let dir = direction_vector(index, direction);
                    let value =
                        trace_interval(scene, pos, dir, t_start, t_end).unwrap_or_else(|| {
                            upper.map_or(config.environment, |upper_level| {
                                // The far field of this ray is the upper
                                // cascade sampled at the interval-end
                                // position, averaged over the two child
                                // directions.
                                let end = Vec2::new(pos.x + dir.x * t_end, pos.y + dir.y * t_end);
                                let (child_a, child_b) = child_directions(direction);
                                let a = sample_level(upper_level, end, child_a);
                                let b = sample_level(upper_level, end, child_b);
                                [
                                    f32::midpoint(a[0], b[0]),
                                    f32::midpoint(a[1], b[1]),
                                    f32::midpoint(a[2], b[2]),
                                ]
                            })
                        });
                    level.radiance[base + direction] = value;
                }
            }
        }
        Ok(level)
    }

    /// The scene this solver solved.
    #[must_use]
    pub const fn scene(&self) -> &RcScene {
        &self.scene
    }

    /// The configuration this solver used.
    #[must_use]
    pub const fn config(&self) -> &RcConfig {
        &self.config
    }

    /// All solved cascade levels, finest first.
    #[must_use]
    pub fn levels(&self) -> &[CascadeLevel] {
        &self.levels
    }

    /// The number of solved cascade levels.
    #[must_use]
    pub const fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Per-probe radiance at cascade-0 resolution, row-major. With
    /// the default base spacing of 1 this is one value per pixel.
    #[must_use]
    pub fn output(&self) -> &[[f32; 3]] {
        &self.output
    }

    /// Width of the output grid (the cascade-0 probe count along x).
    #[must_use]
    pub fn output_width(&self) -> usize {
        self.levels[0].probe_width
    }

    /// Height of the output grid (the cascade-0 probe count along y).
    #[must_use]
    pub fn output_height(&self) -> usize {
        self.levels[0].probe_height
    }

    /// The output radiance at output-grid position `(x, y)`, or
    /// `None` outside the output grid.
    #[must_use]
    pub fn output_pixel(&self, x: usize, y: usize) -> Option<[f32; 3]> {
        if x >= self.output_width() || y >= self.output_height() {
            return None;
        }
        self.output.get(y * self.output_width() + x).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(actual: f32, expected: f32, eps: f32) -> bool {
        (actual - expected).abs() <= eps
    }

    fn approx3(actual: [f32; 3], expected: [f32; 3], eps: f32) -> bool {
        (0..3).all(|c| approx(actual[c], expected[c], eps))
    }

    // ── Geometry: probe grids, directions, intervals ─────────────────

    #[test]
    fn probe_grid_dims_round_up_per_level() {
        assert_eq!(probe_grid_dims(32, 16, 1), (32, 16));
        assert_eq!(probe_grid_dims(32, 16, 2), (16, 8));
        assert_eq!(probe_grid_dims(33, 17, 4), (9, 5));
        assert_eq!(probe_grid_dims(17, 17, 8), (3, 3));
        assert_eq!(probe_grid_dims(10, 10, 0), (0, 0));
    }

    #[test]
    fn direction_counts_double_per_level() {
        assert_eq!(direction_count(0), Some(4));
        assert_eq!(direction_count(1), Some(8));
        assert_eq!(direction_count(2), Some(16));
        assert_eq!(direction_count(3), Some(32));
    }

    #[test]
    fn intervals_are_contiguous_and_cover_from_zero() {
        let mut expected_start = 0.0;
        for level in 0..5 {
            let (t_start, t_end) = ray_interval(level, 1.0);
            assert_eq!(t_start, expected_start, "level {level} does not continue");
            assert!(t_end > t_start, "level {level} has an empty interval");
            expected_start = t_end;
        }
        // Base length 1, growth 4: lengths 1, 4, 16, 64, 256.
        assert_eq!(ray_interval(0, 1.0), (0.0, 1.0));
        assert_eq!(ray_interval(1, 1.0), (1.0, 5.0));
        assert_eq!(ray_interval(2, 1.0), (5.0, 21.0));
        assert_eq!(ray_interval(3, 1.0), (21.0, 85.0));
        // A different base length scales every boundary.
        assert_eq!(ray_interval(1, 2.0), (2.0, 10.0));
    }

    #[test]
    fn solver_levels_match_the_layout_rules() {
        let scene = RcScene::empty(32, 16).unwrap();
        let mut config = RcConfig::default();
        config.cascade_count = Some(3);
        let solver = RadianceCascades::new(scene, config).unwrap();
        assert_eq!(solver.level_count(), 3);
        let expected = [
            (1, 32, 16, 4, 0.0, 1.0),
            (2, 16, 8, 8, 1.0, 5.0),
            (4, 8, 4, 16, 5.0, 21.0),
        ];
        for (level, &(spacing, pw, ph, dirs, t0, t1)) in solver.levels().iter().zip(&expected) {
            assert_eq!(level.spacing, spacing);
            assert_eq!((level.probe_width, level.probe_height), (pw, ph));
            assert_eq!(level.direction_count, dirs);
            assert_eq!((level.t_start, level.t_end), (t0, t1));
        }
    }

    #[test]
    fn child_directions_bracket_the_parent_angle() {
        assert_eq!(child_directions(0), (0, 1));
        assert_eq!(child_directions(3), (6, 7));
        for level in 0..4 {
            let count = direction_count(level).unwrap();
            let parent_step = TAU / count as f32;
            for j in 0..count {
                let parent = direction_angle(level, j);
                let (child_a, child_b) = child_directions(j);
                let a = direction_angle(level + 1, child_a);
                let b = direction_angle(level + 1, child_b);
                // Children sit one quarter step to each side.
                assert!(
                    approx(parent - a, parent_step * 0.25, 1e-6),
                    "level {level} ray {j}: a={a} parent={parent}"
                );
                assert!(
                    approx(b - parent, parent_step * 0.25, 1e-6),
                    "level {level} ray {j}: b={b} parent={parent}"
                );
            }
        }
    }

    #[test]
    fn default_cascade_count_covers_the_diagonal() {
        // Diagonal of 32x16 is ~35.8: reach 21 (3 levels) is short,
        // reach 85 (4 levels) covers it.
        assert_eq!(default_cascade_count(32, 16, 1.0), 4);
        // Diagonal of 4x3 is exactly 5, the reach of 2 levels.
        assert_eq!(default_cascade_count(4, 3, 1.0), 2);
        assert_eq!(default_cascade_count(1, 1, 1.0), 2);
    }

    // ── SDF ──────────────────────────────────────────────────────────

    #[test]
    fn sdf_of_an_empty_grid_is_infinite() {
        let scene = RcScene::empty(4, 4).unwrap();
        assert_eq!(scene.signed_distance(Vec2::new(2.0, 2.0)), f32::INFINITY);
        assert_eq!(scene.signed_distance(Vec2::new(0.5, 0.5)), f32::INFINITY);
    }

    #[test]
    fn sdf_matches_hand_computed_distances() {
        // One solid cell at (2, 1): the square [2, 3) x [1, 2).
        let mut scene = RcScene::empty(5, 4).unwrap();
        scene.set_cell(2, 1, RcCell::Occluder).unwrap();
        // Left of the cell, on its row: 2 - 0.5 = 1.5.
        assert!(approx(
            scene.signed_distance(Vec2::new(0.5, 1.5)),
            1.5,
            1e-6
        ));
        // Below the cell, on its column: 1 - 0.5 = 0.5.
        assert!(approx(
            scene.signed_distance(Vec2::new(2.5, 0.5)),
            0.5,
            1e-6
        ));
        // Diagonal to the corner (2, 1): sqrt(0.5^2 + 0.5^2).
        let d = scene.signed_distance(Vec2::new(1.5, 0.5));
        assert!(approx(d, std::f32::consts::FRAC_1_SQRT_2, 1e-6), "d={d}");
        // The centre of the cell is half a cell deep: -0.5.
        assert!(approx(
            scene.signed_distance(Vec2::new(2.5, 1.5)),
            -0.5,
            1e-6
        ));
        // Off-centre inside: depth is the distance to the nearest face.
        assert!(approx(
            scene.signed_distance(Vec2::new(2.25, 1.5)),
            -0.25,
            1e-6
        ));
    }

    #[test]
    fn sdf_treats_emitters_and_occluders_as_solid() {
        let mut scene = RcScene::empty(6, 3).unwrap();
        scene.set_cell(1, 1, RcCell::Occluder).unwrap();
        scene
            .set_cell(4, 1, RcCell::Emitter([1.0, 0.0, 0.0]))
            .unwrap();
        // Point between them is closer to the occluder (0.5) than to
        // the emitter (1.5).
        assert!(approx(
            scene.signed_distance(Vec2::new(2.5, 1.5)),
            0.5,
            1e-6
        ));
        assert!(approx(
            scene.signed_distance(Vec2::new(3.5, 1.5)),
            0.5,
            1e-6
        ));
    }

    // ── Tracing ──────────────────────────────────────────────────────

    /// 16x8 scene with an emitter at (8, 4), nothing else.
    fn emitter_scene() -> RcScene {
        let mut scene = RcScene::empty(16, 8).unwrap();
        scene
            .set_cell(8, 4, RcCell::Emitter([0.2, 0.6, 1.0]))
            .unwrap();
        scene
    }

    #[test]
    fn trace_hits_an_emitter_inside_the_interval() {
        let scene = emitter_scene();
        let hit = trace_interval(&scene, Vec2::new(2.5, 4.5), Vec2::X, 0.0, 10.0);
        assert_eq!(hit, Some([0.2, 0.6, 1.0]));
        // The same ray starting partway, inside a later interval.
        let hit = trace_interval(&scene, Vec2::new(2.5, 4.5), Vec2::X, 3.0, 10.0);
        assert_eq!(hit, Some([0.2, 0.6, 1.0]));
    }

    #[test]
    fn trace_misses_an_emitter_beyond_the_interval_end() {
        let scene = emitter_scene();
        // The emitter face is 5.5 cells away; the interval ends at 4.
        let hit = trace_interval(&scene, Vec2::new(2.5, 4.5), Vec2::X, 0.0, 4.0);
        assert_eq!(hit, None);
        // A ray in the other direction also misses.
        let hit = trace_interval(&scene, Vec2::new(2.5, 4.5), Vec2::new(-1.0, 0.0), 0.0, 10.0);
        assert_eq!(hit, None);
    }

    #[test]
    fn trace_occluder_blocks_the_emitter_behind_it() {
        let mut scene = emitter_scene();
        scene.set_cell(5, 4, RcCell::Occluder).unwrap();
        let hit = trace_interval(&scene, Vec2::new(2.5, 4.5), Vec2::X, 0.0, 10.0);
        assert_eq!(hit, Some([0.0, 0.0, 0.0]));
    }

    #[test]
    fn trace_edge_cases_miss_cleanly() {
        let scene = emitter_scene();
        // Empty interval.
        assert_eq!(
            trace_interval(&scene, Vec2::new(2.5, 4.5), Vec2::X, 2.0, 2.0),
            None
        );
        // Zero direction.
        assert_eq!(
            trace_interval(&scene, Vec2::new(2.5, 4.5), Vec2::ZERO, 0.0, 5.0),
            None
        );
        // Empty scene: no hit at any range.
        let empty = RcScene::empty(8, 8).unwrap();
        assert_eq!(
            trace_interval(&empty, Vec2::new(4.5, 4.5), Vec2::X, 0.0, 100.0),
            None
        );
    }

    #[test]
    fn trace_starting_inside_an_emitter_hits_it() {
        let scene = emitter_scene();
        let hit = trace_interval(&scene, Vec2::new(8.5, 4.5), Vec2::X, 0.0, 5.0);
        assert_eq!(hit, Some([0.2, 0.6, 1.0]));
    }

    // ── Validation ───────────────────────────────────────────────────

    #[test]
    fn scene_validation_rejects_bad_input() {
        assert_eq!(RcScene::empty(0, 4).unwrap_err(), RcError::ZeroWidth);
        assert_eq!(RcScene::empty(4, 0).unwrap_err(), RcError::ZeroHeight);
        let err = RcScene::new(3, 3, vec![RcCell::Empty; 8]).unwrap_err();
        assert_eq!(
            err,
            RcError::CellCountMismatch {
                expected: 9,
                actual: 8
            }
        );
        let bad = vec![RcCell::Emitter([f32::NAN, 0.0, 0.0]); 1];
        assert_eq!(
            RcScene::new(1, 1, bad).unwrap_err(),
            RcError::NonFiniteRadiance
        );
        let bad = vec![RcCell::Emitter([-1.0, 0.0, 0.0]); 1];
        assert_eq!(
            RcScene::new(1, 1, bad).unwrap_err(),
            RcError::NegativeRadiance
        );
    }

    #[test]
    fn config_validation_rejects_bad_input() {
        let scene = || RcScene::empty(4, 4).unwrap();
        let mut config = RcConfig {
            base_spacing: 0,
            ..RcConfig::default()
        };
        assert_eq!(
            RadianceCascades::new(scene(), config.clone()).unwrap_err(),
            RcError::ZeroBaseSpacing
        );
        config.base_spacing = 1;
        config.base_interval_length = 0.0;
        assert_eq!(
            RadianceCascades::new(scene(), config.clone()).unwrap_err(),
            RcError::InvalidBaseIntervalLength
        );
        config.base_interval_length = 1.0;
        config.cascade_count = Some(0);
        assert_eq!(
            RadianceCascades::new(scene(), config.clone()).unwrap_err(),
            RcError::ZeroCascadeCount
        );
        config.cascade_count = Some(MAX_CASCADES + 1);
        assert_eq!(
            RadianceCascades::new(scene(), config.clone()).unwrap_err(),
            RcError::TooManyCascades {
                requested: MAX_CASCADES + 1
            }
        );
        config.cascade_count = None;
        config.environment = [0.0, f32::INFINITY, 0.0];
        assert_eq!(
            RadianceCascades::new(scene(), config).unwrap_err(),
            RcError::NonFiniteRadiance
        );
    }

    #[test]
    fn set_cell_checks_bounds_and_keeps_the_solid_list() {
        let mut scene = RcScene::empty(3, 3).unwrap();
        assert_eq!(
            scene.set_cell(3, 0, RcCell::Occluder).unwrap_err(),
            RcError::OutOfBounds { x: 3, y: 0 }
        );
        scene.set_cell(1, 1, RcCell::Occluder).unwrap();
        assert!(approx(
            scene.signed_distance(Vec2::new(1.5, 0.5)),
            0.5,
            1e-6
        ));
        // Clearing the cell removes it from the distance field again.
        scene.set_cell(1, 1, RcCell::Empty).unwrap();
        assert_eq!(scene.signed_distance(Vec2::new(1.5, 0.5)), f32::INFINITY);
    }

    // ── Bilinear sampling ────────────────────────────────────────────

    #[test]
    fn bilinear_sample_is_exact_at_probes_and_averages_between() {
        let mut level = CascadeLevel {
            index: 1,
            spacing: 2,
            probe_width: 2,
            probe_height: 1,
            direction_count: 1,
            t_start: 1.0,
            t_end: 5.0,
            radiance: vec![[0.0, 0.0, 0.0], [1.0, 0.5, 0.25]],
        };
        // At a probe position the sample is that probe, exactly.
        assert_eq!(
            sample_level(&level, Vec2::new(2.5, 0.5), 0),
            [1.0, 0.5, 0.25]
        );
        // Halfway between the probes it is the plain average.
        let mid = sample_level(&level, Vec2::new(1.5, 0.5), 0);
        assert!(approx3(mid, [0.5, 0.25, 0.125], 1e-6), "mid={mid:?}");
        // A constant field samples back exactly, at any position.
        level.radiance = vec![[0.3, 0.6, 0.9]; 2];
        assert_eq!(
            sample_level(&level, Vec2::new(1.3, 0.9), 0),
            [0.3, 0.6, 0.9]
        );
    }

    // ── End to end: exact cases ──────────────────────────────────────

    #[test]
    fn empty_scene_returns_the_environment_exactly() {
        let scene = RcScene::empty(12, 9).unwrap();
        let mut config = RcConfig::default();
        config.environment = [0.3, 0.5, 0.7];
        let solver = RadianceCascades::new(scene, config).unwrap();
        assert_eq!(solver.output().len(), 12 * 9);
        for (i, px) in solver.output().iter().enumerate() {
            assert_eq!(*px, [0.3, 0.5, 0.7], "pixel {i}");
        }
    }

    #[test]
    fn fully_emissive_scene_returns_the_emission_exactly() {
        let color = [0.9, 0.4, 0.1];
        let scene = RcScene::new(8, 6, vec![RcCell::Emitter(color); 48]).unwrap();
        let solver = RadianceCascades::new(scene, RcConfig::default()).unwrap();
        assert!(solver.output().iter().all(|&px| px == color));
    }

    #[test]
    fn solid_pixels_return_their_own_emission() {
        let mut scene = RcScene::empty(7, 5).unwrap();
        scene
            .set_cell(3, 2, RcCell::Emitter([1.0, 0.5, 0.25]))
            .unwrap();
        scene.set_cell(1, 1, RcCell::Occluder).unwrap();
        let solver = RadianceCascades::new(scene, RcConfig::default()).unwrap();
        assert_eq!(solver.output_pixel(3, 2), Some([1.0, 0.5, 0.25]));
        assert_eq!(solver.output_pixel(1, 1), Some([0.0, 0.0, 0.0]));
        assert_eq!(solver.output_pixel(99, 99), None);
    }

    #[test]
    fn wall_blocks_all_light_to_the_left_side() {
        // A full-height occluder wall splits the scene. The only
        // emitter sits on the right and the environment is black,
        // so the left side must be exactly black.
        //
        // The wall is 24 cells thick on purpose. An upper cascade
        // ray does not start at its probe: the highest cascade here
        // starts its interval at t = 21, so a one-cell wall can be
        // jumped entirely — the ray segment begins on the far side
        // and never sees the wall (the known thin-wall leak of the
        // cascade split, seen in an earlier version of this test
        // with a one-cell wall). A wall thicker than the largest
        // interval start cannot be jumped: every left-side ray
        // segment either starts inside the wall (a hit, black) or
        // crosses its left face (also a hit), and every far-field
        // sample that reaches past the left face lands on wall
        // probes, which are black. Pixels at x <= 8 are asserted:
        // they sit at least 7.5 cells left of the wall face.
        let mut scene = RcScene::empty(56, 16).unwrap();
        for y in 0..16 {
            for x in 16..40 {
                scene.set_cell(x, y, RcCell::Occluder).unwrap();
            }
        }
        scene
            .set_cell(48, 8, RcCell::Emitter([1.0, 0.8, 0.2]))
            .unwrap();
        let solver = RadianceCascades::new(scene, RcConfig::default()).unwrap();
        for y in 0..16 {
            for x in 0..=8 {
                assert_eq!(
                    solver.output_pixel(x, y),
                    Some([0.0, 0.0, 0.0]),
                    "pixel ({x}, {y}) leaked light through the wall"
                );
            }
        }
        // Right of the wall, near the emitter, light arrives.
        let lit = solver.output_pixel(44, 8).unwrap();
        assert!(lit[0] > 0.0 && lit[1] > 0.0, "right side is dark: {lit:?}");
        let near = solver.output_pixel(42, 8).unwrap();
        assert!(near[0] > 0.0, "pixel next to the wall is dark: {near:?}");
    }

    // ── End to end: properties ───────────────────────────────────────

    /// A tiny deterministic linear congruential generator for the
    /// property tests: no new dependencies, same seed, same scene.
    struct Lcg(u32);

    impl Lcg {
        fn next(&mut self) -> u32 {
            self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            self.0 >> 8
        }

        fn unit(&mut self) -> f32 {
            self.next() as f32 / (1u32 << 24) as f32
        }
    }

    #[test]
    fn random_scenes_stay_within_the_radiance_bounds() {
        let mut rng = Lcg(0x1234_5678);
        for trial in 0..12 {
            let (w, h) = (12, 10);
            let mut cells = Vec::with_capacity(w * h);
            let mut max_emission = 0.0f32;
            for _ in 0..w * h {
                let pick = rng.unit();
                let cell = if pick < 0.25 {
                    RcCell::Occluder
                } else if pick < 0.40 {
                    let value = [rng.unit() * 2.0, rng.unit() * 2.0, rng.unit() * 2.0];
                    max_emission = max_emission.max(value[0]).max(value[1]).max(value[2]);
                    RcCell::Emitter(value)
                } else {
                    RcCell::Empty
                };
                cells.push(cell);
            }
            let scene = RcScene::new(w, h, cells).unwrap();
            let mut config = RcConfig::default();
            config.environment = [0.1, 0.2, 0.3];
            config.cascade_count = Some(3);
            let bound = max_emission.max(0.3);
            let solver = RadianceCascades::new(scene, config).unwrap();
            for (i, px) in solver.output().iter().enumerate() {
                for channel in 0..3 {
                    let v = px[channel];
                    assert!(v.is_finite(), "trial {trial} pixel {i}: {v}");
                    // Convex averages may round one step past the
                    // bound; the tolerance covers only that rounding.
                    assert!(
                        (-1e-5..=bound + 1e-5).contains(&v),
                        "trial {trial} pixel {i} channel {channel}: {v} outside [0, {bound}]"
                    );
                }
            }
        }
    }

    #[test]
    fn centred_emitter_output_is_symmetric() {
        // 17 cells is 2^4 + 1: every power-of-two probe grid anchored
        // at the left edge is also anchored at the right edge, so
        // the probe layouts mirror exactly and only float rounding
        // breaks the symmetry.
        let mut scene = RcScene::empty(17, 17).unwrap();
        scene
            .set_cell(8, 8, RcCell::Emitter([1.2, 0.9, 0.6]))
            .unwrap();
        let solver = RadianceCascades::new(scene, RcConfig::default()).unwrap();
        let mut worst = 0.0f32;
        for y in 0..17 {
            for x in 0..17 {
                let here = solver.output_pixel(x, y).unwrap();
                let flip_x = solver.output_pixel(16 - x, y).unwrap();
                let flip_y = solver.output_pixel(x, 16 - y).unwrap();
                for channel in 0..3 {
                    worst = worst.max((here[channel] - flip_x[channel]).abs());
                    worst = worst.max((here[channel] - flip_y[channel]).abs());
                }
            }
        }
        assert!(worst < 1e-3, "worst symmetry error {worst}");
        // And the centre outshines a corner: light actually spread.
        let centre = solver.output_pixel(8, 8).unwrap();
        let corner = solver.output_pixel(0, 0).unwrap();
        assert_eq!(centre, [1.2, 0.9, 0.6]);
        assert!(corner[0] > 0.0 && corner[0] < centre[0]);
    }

    #[test]
    fn coarse_base_spacing_outputs_at_probe_resolution() {
        let scene = RcScene::empty(16, 8).unwrap();
        let mut config = RcConfig::default();
        config.base_spacing = 2;
        config.environment = [0.2, 0.2, 0.2];
        let solver = RadianceCascades::new(scene, config).unwrap();
        assert_eq!((solver.output_width(), solver.output_height()), (8, 4));
        assert_eq!(solver.output().len(), 32);
        assert!(solver.output().iter().all(|&px| px == [0.2, 0.2, 0.2]));
    }
}
