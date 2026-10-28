//! # Jump-Flood Signed Distance Fields and Irradiance Probes 🔦
//!
//! Analytic SDFs ([`crate::sdf`]) are exact, but they describe a scene that
//! is fixed in code. This module builds a signed distance field from a
//! **rasterized occupancy grid** instead — walls, sprites, tile maps,
//! destructible terrain, anything that can answer "is this cell solid?" —
//! so the field can be rebuilt every frame, or only when the scene changes.
//! On top of the field sit the two consumers that make it a lighting
//! system: sphere-traced shadow rays and SDFDDGI-style irradiance probes.
//! Together they give dynamic global illumination with no ray-tracing
//! hardware: move a wall, rebuild the field, and the light bounces move.
//!
//! ## The jump flood algorithm (JFA)
//!
//! The field is computed with the jump flood algorithm (Rong & Tan 2006).
//! Every solid cell starts as a *seed* carrying its own coordinates. In a
//! series of passes with step sizes halving from the largest power of two
//! below the grid size down to 1, each cell adopts the nearest seed among
//! the 3×3 neighbourhood of cells at the current step (3×3×3 in 3D). Two
//! extra step-1 sweeps (the "JFA+1" tail) clean up the cases the halving
//! schedule strands. The same flood is run twice — once seeded by solid
//! cells, once by empty cells — and the signed distance combines both:
//!
//! ```text
//! signed(p) =  0.5 − dist_to_nearest_empty(p)   if p is solid
//!             −0.5 + dist_to_nearest_solid(p)   if p is empty
//! ```
//!
//! Distances are measured between cell centres, so the zero crossing sits
//! half a cell outside a solid cell's centre — for an axis-aligned face
//! this is the exact distance to the face. Diagonal and corner distances
//! are the Euclidean distance to the nearest opposite cell centre minus
//! half a cell, the standard grid-SDF approximation.
//!
//! Two invariants hold by construction, and the tests pin both:
//!
//! 1. The stored nearest seed is always a real seed cell, so the reported
//!    distance is the distance to an actual solid cell — the field never
//!    *underestimates* the true nearest-seed distance. (JFA is an
//!    approximation scheme in the literature; with the extra step-1
//!    sweeps it matched a brute-force nearest-seed search exactly on
//!    every structured and randomized grid in this module's test suite,
//!    but the guaranteed property is the one-sided bound.)
//! 2. If one side has no cells at all (empty or completely solid grid),
//!    its distance saturates at the grid diagonal instead of infinity, so
//!    sphere tracing still takes finite, terminating steps.
//!
//! Coordinates: grid point `(x, y)` sits at the integer position
//! `(x, y)` in *texel units*; one texel is one cell. Continuous queries
//! ([`JumpFloodSdf2D::sample`]) bilinearly interpolate the grid (trilinear
//! in 3D) and clamp to the border.
//!
//! ## Probes (SDFDDGI-style)
//!
//! [`ProbeGrid2D`] places irradiance probes on a regular 2D grid. On
//! [`update`](ProbeGrid2D::update) each probe sphere-traces a fan of rays
//! against the field; a hit samples a caller-supplied surface-radiance
//! function, a miss samples the sky colour. Each probe stores the mean
//! radiance, the mean hit distance, and the mean squared hit distance —
//! the first two moments DDGI keeps. Querying with
//! [`sample`](ProbeGrid2D::sample) blends the four surrounding probes with
//! bilinear weights multiplied by a Chebyshev visibility weight computed
//! from those moments, so a probe on the far side of a wall contributes
//! almost nothing to a point in front of it even though the wall is not
//! stored anywhere in the probe data.
//!
//! ## Examples
//!
//! ```rust
//! use abrash_core::jump_flood::{JumpFloodSdf2D, ProbeGrid2D};
//! use abrash_core::color::Color;
//! use abrash_core::math::Vec2;
//!
//! // A 32×32 room with a solid block in the middle.
//! let mut occ = vec![false; 32 * 32];
//! for y in 12..20 {
//!     for x in 12..20 {
//!         occ[y * 32 + x] = true;
//!     }
//! }
//! let field = JumpFloodSdf2D::from_occupancy(32, 32, &occ);
//!
//! // The block's centre is deep inside; the corner cell is far outside.
//! assert!(field.signed_distance(16, 16) < -3.0);
//! assert!(field.signed_distance(0, 0) > 10.0);
//!
//! // A shadow ray from the left wall reaches a point left of the block…
//! let lit = field.shadow_visibility(Vec2::new(6.0, 16.0), Vec2::new(2.0, 16.0), 8.0);
//! assert_eq!(lit, 1.0);
//! // …but not a point hidden behind the block.
//! let shaded = field.shadow_visibility(Vec2::new(26.0, 16.0), Vec2::new(2.0, 16.0), 8.0);
//! assert_eq!(shaded, 0.0);
//!
//! // Probes in an empty-coloured sky: every probe sees only sky.
//! let mut probes = ProbeGrid2D::new(Vec2::ZERO, 8.0, 4, 4, 16);
//! probes.update(&field, 64.0, Color::rgb(0.2, 0.3, 0.5), &|_, _| Color::WHITE);
//! let irr = probes.sample(Vec2::new(16.0, 16.0));
//! assert!(irr.r > 0.19 && irr.g > 0.29); // sky where rays miss, walls where they hit
//! ```

use crate::color::Color;
use crate::math::{Vec2, Vec3};

/// Smallest field value a sphere trace treats as "on the surface",
/// in texels.
pub const TRACE_HIT_EPSILON: f32 = 0.05;

/// Hard cap on sphere-trace steps; guarantees termination even where the
/// field is nearly tangent to the ray.
pub const MAX_TRACE_STEPS: u32 = 512;

/// Default number of rays each irradiance probe casts per update.
pub const DEFAULT_RAYS_PER_PROBE: u32 = 32;

// ── Shared helpers ──────────────────────────────────────────────────────────

/// FNV-1a hash of grid dimensions and occupancy, for change detection.
fn occupancy_hash(width: u32, height: u32, depth: u32, occupancy: &[bool]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in width
        .to_le_bytes()
        .iter()
        .chain(height.to_le_bytes().iter())
        .chain(depth.to_le_bytes().iter())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    for cell in occupancy {
        hash ^= u64::from(*cell);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// JFA pass schedule: halving steps from the largest power of two below
/// `max_dim`, then two extra step-1 sweeps.
fn jfa_steps(max_dim: u32) -> Vec<i32> {
    let mut pow2: u32 = 1;
    while pow2 < max_dim {
        pow2 *= 2;
    }
    let mut steps = Vec::new();
    let mut step = (pow2 / 2) as i32;
    while step >= 1 {
        steps.push(step);
        step /= 2;
    }
    steps.push(1);
    steps.push(1);
    steps
}

fn normalize2(v: Vec2) -> Vec2 {
    let len = v.length();
    if len > 1e-8 {
        v * (1.0 / len)
    } else {
        Vec2::ZERO
    }
}

fn normalize3(v: Vec3) -> Vec3 {
    let len = v.length();
    if len > 1e-8 {
        v * (1.0 / len)
    } else {
        Vec3::ZERO
    }
}

/// Rasterizes a 2D occupancy grid from an analytic signed-distance
/// function: cell `(x, y)` is solid iff `sdf` at the cell's integer
/// position is `<= 0.0`.
///
/// # Panics
///
/// Panics if the grid size overflows.
#[must_use]
pub fn occupancy_from_sdf_2d(width: u32, height: u32, sdf: impl Fn(Vec2) -> f32) -> Vec<bool> {
    let size = usize::try_from(u64::from(width) * u64::from(height))
        .expect("occupancy grid dimensions overflow");
    let mut occ = vec![false; size];
    for y in 0..height {
        for x in 0..width {
            let p = Vec2::new(x as f32, y as f32);
            occ[(y * width + x) as usize] = sdf(p) <= 0.0;
        }
    }
    occ
}

/// Rasterizes a 3D occupancy grid from an analytic signed-distance
/// function: voxel `(x, y, z)` is solid iff `sdf` at the voxel's integer
/// position is `<= 0.0`.
///
/// # Panics
///
/// Panics if the grid size overflows.
#[must_use]
pub fn occupancy_from_sdf_3d(
    width: u32,
    height: u32,
    depth: u32,
    sdf: impl Fn(Vec3) -> f32,
) -> Vec<bool> {
    let size = usize::try_from(u64::from(width) * u64::from(height) * u64::from(depth))
        .expect("occupancy grid dimensions overflow");
    let mut occ = vec![false; size];
    for z in 0..depth {
        for y in 0..height {
            for x in 0..width {
                let p = Vec3::new(x as f32, y as f32, z as f32);
                occ[((z * height + y) * width + x) as usize] = sdf(p) <= 0.0;
            }
        }
    }
    occ
}

// ── 2D field ────────────────────────────────────────────────────────────────

/// The result of a successful sphere trace through a 2D field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceHit {
    /// Distance travelled along the ray, in texels.
    pub distance: f32,
    /// World (texel-space) position where the ray stopped.
    pub point: Vec2,
    /// Number of sphere-trace steps taken.
    pub steps: u32,
}

/// A 2D signed distance field built from an occupancy grid with the
/// jump flood algorithm. See the module documentation for conventions
/// and invariants.
///
/// Typical per-frame flow:
///
/// 1. Construct once with [`new`](Self::new) (or build directly with
///    [`from_occupancy`](Self::from_occupancy)).
/// 2. Each frame call [`rebuild_if_changed`](Self::rebuild_if_changed)
///    with the current occupancy; the flood re-runs only when the
///    occupancy hash differs, and the buffers are reused either way.
/// 3. Query with [`signed_distance`](Self::signed_distance),
///    [`sample`](Self::sample), [`trace`](Self::trace), and
///    [`shadow_visibility`](Self::shadow_visibility).
pub struct JumpFloodSdf2D {
    width: u32,
    height: u32,
    signed: Vec<f32>,
    nearest_seed: Vec<i32>,
    hash: u64,
}

impl JumpFloodSdf2D {
    /// Creates an empty field of the given size (all distances saturate
    /// at the grid diagonal until the first rebuild).
    ///
    /// # Panics
    ///
    /// Panics if a dimension is zero or the grid size overflows.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0, "field dimensions must be non-zero");
        let size = usize::try_from(u64::from(width) * u64::from(height))
            .expect("field dimensions overflow");
        let diag = ((width * width + height * height) as f32).sqrt();
        Self {
            width,
            height,
            signed: vec![diag - 0.5; size],
            nearest_seed: vec![-1; size],
            hash: 0,
        }
    }

    /// Builds a field directly from an occupancy grid
    /// (`occupancy[y * width + x]`, `true` = solid).
    ///
    /// # Panics
    ///
    /// Panics if a dimension is zero or `occupancy.len() != width * height`.
    #[must_use]
    pub fn from_occupancy(width: u32, height: u32, occupancy: &[bool]) -> Self {
        let mut field = Self::new(width, height);
        field.rebuild(occupancy);
        field
    }

    /// Width of the field in texels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height of the field in texels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Rebuilds the field from `occupancy`, reusing the existing buffers.
    ///
    /// # Panics
    ///
    /// Panics if `occupancy.len() != width * height`.
    pub fn rebuild(&mut self, occupancy: &[bool]) {
        let expected = (self.width * self.height) as usize;
        assert_eq!(
            occupancy.len(),
            expected,
            "occupancy length must equal width * height"
        );
        self.hash = occupancy_hash(self.width, self.height, 1, occupancy);
        let seeds_solid: Vec<bool> = occupancy.to_vec();
        let seeds_empty: Vec<bool> = occupancy.iter().map(|c| !c).collect();
        let nearest_solid = jfa_nearest_2d(self.width, self.height, &seeds_solid);
        let nearest_empty = jfa_nearest_2d(self.width, self.height, &seeds_empty);
        let w = self.width;
        let diag = ((w * w + self.height * self.height) as f32).sqrt();
        for (i, cell) in self.signed.iter_mut().enumerate() {
            let x = (i as u32) % w;
            let y = (i as u32) / w;
            *cell = if occupancy[i] {
                0.5 - dist_to_seed_2d(x, y, nearest_empty[i], w, diag)
            } else {
                dist_to_seed_2d(x, y, nearest_solid[i], w, diag) - 0.5
            };
        }
        self.nearest_seed = nearest_solid;
    }

    /// Rebuilds only if `occupancy` differs from the occupancy used for
    /// the last build. Returns `true` if a rebuild happened.
    ///
    /// This is the "rebuild on change" path: a dynamic scene can hand
    /// its occupancy in every frame and pay the flood cost only on the
    /// frames where something actually moved.
    ///
    /// # Panics
    ///
    /// Panics if `occupancy.len() != width * height`.
    pub fn rebuild_if_changed(&mut self, occupancy: &[bool]) -> bool {
        let expected = (self.width * self.height) as usize;
        assert_eq!(
            occupancy.len(),
            expected,
            "occupancy length must equal width * height"
        );
        if occupancy_hash(self.width, self.height, 1, occupancy) == self.hash && self.hash != 0 {
            return false;
        }
        self.rebuild(occupancy);
        true
    }

    /// Signed distance at grid point `(x, y)`: negative inside solid,
    /// positive outside. Out-of-bounds coordinates clamp to the border.
    #[must_use]
    pub fn signed_distance(&self, x: u32, y: u32) -> f32 {
        let cx = x.min(self.width - 1);
        let cy = y.min(self.height - 1);
        self.signed[(cy * self.width + cx) as usize]
    }

    /// The whole signed-distance grid, row-major.
    #[must_use]
    pub fn distances(&self) -> &[f32] {
        &self.signed
    }

    /// The nearest solid cell to `(x, y)`, or `None` if the grid has no
    /// solid cells. Out-of-bounds coordinates clamp to the border.
    #[must_use]
    pub fn nearest_seed(&self, x: u32, y: u32) -> Option<(u32, u32)> {
        let cx = x.min(self.width - 1);
        let cy = y.min(self.height - 1);
        let seed = self.nearest_seed[(cy * self.width + cx) as usize];
        if seed < 0 {
            None
        } else {
            let s = seed as u32;
            Some((s % self.width, s / self.width))
        }
    }

    /// Bilinearly interpolated signed distance at continuous position
    /// `p` (texel units). Positions outside the grid clamp to the border.
    #[must_use]
    pub fn sample(&self, p: Vec2) -> f32 {
        let fx = p.x.clamp(0.0, (self.width - 1) as f32);
        let fy = p.y.clamp(0.0, (self.height - 1) as f32);
        let x0 = fx.floor() as u32;
        let y0 = fy.floor() as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let d00 = self.signed[(y0 * self.width + x0) as usize];
        let d10 = self.signed[(y0 * self.width + x1) as usize];
        let d01 = self.signed[(y1 * self.width + x0) as usize];
        let d11 = self.signed[(y1 * self.width + x1) as usize];
        let top = d00 + (d10 - d00) * tx;
        let bottom = d01 + (d11 - d01) * tx;
        top + (bottom - top) * ty
    }

    /// Approximate surface normal at `p`: the normalized gradient of the
    /// sampled field, pointing away from the nearest surface. Returns
    /// [`Vec2::ZERO`] where the field is locally flat (deep inside or
    /// far outside with no gradient information).
    #[must_use]
    pub fn normal_at(&self, p: Vec2) -> Vec2 {
        let e = 0.5;
        let gx = self.sample(p + Vec2::new(e, 0.0)) - self.sample(p - Vec2::new(e, 0.0));
        let gy = self.sample(p + Vec2::new(0.0, e)) - self.sample(p - Vec2::new(0.0, e));
        normalize2(Vec2::new(gx, gy))
    }

    /// Sphere-traces (circle-traces in 2D) a ray against the field.
    ///
    /// `direction` need not be normalized. Returns the first hit within
    /// `max_distance`, or `None` if the ray leaves the field's influence
    /// or exceeds the step cap. A ray starting inside solid reports a
    /// hit at distance 0.
    #[must_use]
    pub fn trace(&self, origin: Vec2, direction: Vec2, max_distance: f32) -> Option<TraceHit> {
        if max_distance <= 0.0 || direction.length_sq() < 1e-12 {
            return None;
        }
        let dir = normalize2(direction);
        let mut t = 0.0f32;
        let mut steps = 0u32;
        loop {
            let p = origin + dir * t;
            let d = self.sample(p);
            steps += 1;
            if d <= TRACE_HIT_EPSILON {
                return Some(TraceHit {
                    distance: t,
                    point: p,
                    steps,
                });
            }
            t += d;
            if t > max_distance || steps >= MAX_TRACE_STEPS {
                return None;
            }
        }
    }

    /// Sphere-traced soft-shadow visibility of `light` from `point`,
    /// in `[0.0, 1.0]`: 1 = fully lit, 0 = fully occluded or inside
    /// solid, values between = penumbra.
    ///
    /// Uses the classic penumbra estimator (Quilez): while marching
    /// toward the light, track `min(sharpness * distance / travelled)`.
    /// Larger `sharpness` gives harder shadows (16 is a good default).
    #[must_use]
    pub fn shadow_visibility(&self, point: Vec2, light: Vec2, sharpness: f32) -> f32 {
        let to_light = light - point;
        let dist = to_light.length();
        if dist < 1e-6 {
            return 1.0;
        }
        let dir = to_light * (1.0 / dist);
        let mut res = 1.0f32;
        let mut t = 0.0f32;
        for _ in 0..MAX_TRACE_STEPS {
            let p = point + dir * t;
            let d = self.sample(p);
            if d <= TRACE_HIT_EPSILON {
                return 0.0;
            }
            if t > 1e-4 {
                res = res.min(sharpness * d / t);
            }
            t += d;
            if t >= dist {
                break;
            }
        }
        res.clamp(0.0, 1.0)
    }
}

/// One JFA flood over a 2D grid: returns, per cell, the flat index of
/// the nearest seed cell (or −1 where no seed exists).
fn jfa_nearest_2d(width: u32, height: u32, is_seed: &[bool]) -> Vec<i32> {
    let size = (width * height) as usize;
    let mut cur: Vec<i32> = (0..size)
        .map(|i| if is_seed[i] { i as i32 } else { -1 })
        .collect();
    if cur.iter().all(|s| *s < 0) {
        return cur;
    }
    let mut next = cur.clone();
    for step in jfa_steps(width.max(height)) {
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let idx = (y * width as i32 + x) as usize;
                let mut best = cur[idx];
                let mut best_d2 = seed_dist2_2d(x, y, best, width);
                for dy in [-step, 0, step] {
                    let ny = y + dy;
                    if ny < 0 || ny >= height as i32 {
                        continue;
                    }
                    for dx in [-step, 0, step] {
                        let nx = x + dx;
                        if nx < 0 || nx >= width as i32 {
                            continue;
                        }
                        let cand = cur[(ny * width as i32 + nx) as usize];
                        if cand < 0 {
                            continue;
                        }
                        let d2 = seed_dist2_2d(x, y, cand, width);
                        if d2 < best_d2 {
                            best_d2 = d2;
                            best = cand;
                        }
                    }
                }
                next[idx] = best;
            }
        }
        std::mem::swap(&mut cur, &mut next);
    }
    cur
}

/// Squared distance from `(x, y)` to seed flat index `seed`
/// (`i64::MAX` for "no seed").
fn seed_dist2_2d(x: i32, y: i32, seed: i32, width: u32) -> i64 {
    if seed < 0 {
        return i64::MAX;
    }
    let sx = (seed as u32 % width) as i32;
    let sy = (seed as u32 / width) as i32;
    let dx = i64::from(x - sx);
    let dy = i64::from(y - sy);
    dx * dx + dy * dy
}

/// Distance from `(x, y)` to seed flat index `seed`, saturating at
/// `fallback` when there is no seed.
fn dist_to_seed_2d(x: u32, y: u32, seed: i32, width: u32, fallback: f32) -> f32 {
    let d2 = seed_dist2_2d(x as i32, y as i32, seed, width);
    if d2 == i64::MAX {
        fallback
    } else {
        (d2 as f32).sqrt()
    }
}

// ── Irradiance probes (SDFDDGI-style, 2D) ───────────────────────────────────

/// One irradiance probe: mean incoming radiance plus the first two
/// moments of hit distance, exactly the data DDGI stores per probe
/// (here in texel units, uncompressed).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IrradianceProbe {
    /// Probe position in texel units.
    pub position: Vec2,
    /// Mean incoming radiance over the last update's ray fan.
    pub irradiance: Color,
    /// Mean hit distance (misses count as the probe's max distance).
    pub mean_distance: f32,
    /// Mean squared hit distance.
    pub mean_distance_sq: f32,
}

impl IrradianceProbe {
    /// Creates a probe at `position` with no gathered light.
    #[must_use]
    pub const fn new(position: Vec2) -> Self {
        Self {
            position,
            irradiance: Color::BLACK,
            mean_distance: 0.0,
            mean_distance_sq: 0.0,
        }
    }

    /// Chebyshev visibility of this probe from `point`, in `[0, 1]`.
    ///
    /// If `point` is no farther than the probe's mean hit distance the
    /// probe can see it (weight 1). Beyond that, the weight falls off
    /// as `variance / (variance + excess²)` — the Chebyshev upper bound
    /// DDGI uses — so geometry the probe usually hits close-by hides it
    /// from points behind that geometry.
    #[must_use]
    pub fn chebyshev_visibility(&self, point: Vec2) -> f32 {
        let dist = (point - self.position).length();
        if dist <= self.mean_distance {
            return 1.0;
        }
        let variance = (self.mean_distance_sq - self.mean_distance * self.mean_distance).max(0.0);
        let excess = dist - self.mean_distance;
        let denom = variance + excess * excess;
        if denom <= 1e-9 {
            0.0
        } else {
            (variance / denom).clamp(0.0, 1.0)
        }
    }
}

/// A regular 2D grid of [`IrradianceProbe`]s over a
/// [`JumpFloodSdf2D`]. See the module documentation for the model.
pub struct ProbeGrid2D {
    origin: Vec2,
    spacing: f32,
    count_x: u32,
    count_y: u32,
    rays_per_probe: u32,
    probes: Vec<IrradianceProbe>,
}

impl ProbeGrid2D {
    /// Creates a probe grid whose first probe sits at `origin`, with
    /// probes every `spacing` texels, `count_x` × `count_y` probes, each
    /// casting `rays_per_probe` rays per update.
    ///
    /// # Panics
    ///
    /// Panics if a count is zero, `spacing <= 0`, or `rays_per_probe`
    /// is zero.
    #[must_use]
    pub fn new(
        origin: Vec2,
        spacing: f32,
        count_x: u32,
        count_y: u32,
        rays_per_probe: u32,
    ) -> Self {
        assert!(count_x > 0 && count_y > 0, "probe counts must be non-zero");
        assert!(spacing > 0.0, "probe spacing must be positive");
        assert!(rays_per_probe > 0, "rays_per_probe must be non-zero");
        let mut probes = Vec::with_capacity((count_x * count_y) as usize);
        for iy in 0..count_y {
            for ix in 0..count_x {
                probes.push(IrradianceProbe::new(Vec2::new(
                    origin.x + ix as f32 * spacing,
                    origin.y + iy as f32 * spacing,
                )));
            }
        }
        Self {
            origin,
            spacing,
            count_x,
            count_y,
            rays_per_probe,
            probes,
        }
    }

    /// All probes, row-major (`probes[iy * count_x + ix]`).
    #[must_use]
    pub fn probes(&self) -> &[IrradianceProbe] {
        &self.probes
    }

    /// The probe at grid coordinate `(ix, iy)`, if in range.
    #[must_use]
    pub fn probe(&self, ix: u32, iy: u32) -> Option<&IrradianceProbe> {
        if ix < self.count_x && iy < self.count_y {
            Some(&self.probes[(iy * self.count_x + ix) as usize])
        } else {
            None
        }
    }

    /// Re-gathers every probe against `field`.
    ///
    /// Each probe casts `rays_per_probe` evenly spaced rays. A ray that
    /// hits within `max_distance` samples
    /// `surface_radiance(hit_point, surface_normal)`; a miss contributes
    /// `sky`. The probe's irradiance and distance moments are the plain
    /// means over the fan (the surface function is expected to return
    /// radiance already weighted as the caller wants it).
    pub fn update(
        &mut self,
        field: &JumpFloodSdf2D,
        max_distance: f32,
        sky: Color,
        surface_radiance: &dyn Fn(Vec2, Vec2) -> Color,
    ) {
        let rays = self.rays_per_probe;
        for probe in &mut self.probes {
            let mut sum = [0.0f32; 3];
            let mut sum_d = 0.0f32;
            let mut sum_d2 = 0.0f32;
            for i in 0..rays {
                let angle = std::f32::consts::TAU * (i as f32 + 0.5) / rays as f32;
                let dir = Vec2::from_angle(angle);
                let (radiance, dist) = field.trace(probe.position, dir, max_distance).map_or(
                    (sky, max_distance),
                    |hit| {
                        (
                            surface_radiance(hit.point, field.normal_at(hit.point)),
                            hit.distance,
                        )
                    },
                );
                sum[0] += radiance.r;
                sum[1] += radiance.g;
                sum[2] += radiance.b;
                sum_d += dist;
                sum_d2 += dist * dist;
            }
            let inv = 1.0 / rays as f32;
            probe.irradiance = Color::rgb(sum[0] * inv, sum[1] * inv, sum[2] * inv);
            probe.mean_distance = sum_d * inv;
            probe.mean_distance_sq = sum_d2 * inv;
        }
    }

    /// Samples the irradiance at `point`: bilinear blend of the four
    /// surrounding probes, each weighted by its
    /// [`chebyshev_visibility`](IrradianceProbe::chebyshev_visibility).
    /// Outside the grid the sample clamps to the border probes. If every
    /// weight vanishes (all probes occluded from here), the nearest
    /// probe's irradiance is returned so the result is never NaN.
    #[must_use]
    pub fn sample(&self, point: Vec2) -> Color {
        let gx = ((point.x - self.origin.x) / self.spacing).clamp(0.0, (self.count_x - 1) as f32);
        let gy = ((point.y - self.origin.y) / self.spacing).clamp(0.0, (self.count_y - 1) as f32);
        let x0 = gx.floor() as u32;
        let y0 = gy.floor() as u32;
        let x1 = (x0 + 1).min(self.count_x - 1);
        let y1 = (y0 + 1).min(self.count_y - 1);
        let fx = gx - x0 as f32;
        let fy = gy - y0 as f32;
        let corners = [
            (x0, y0, (1.0 - fx) * (1.0 - fy)),
            (x1, y0, fx * (1.0 - fy)),
            (x0, y1, (1.0 - fx) * fy),
            (x1, y1, fx * fy),
        ];
        let mut sum = [0.0f32; 3];
        let mut total = 0.0f32;
        for (ix, iy, bilinear) in corners {
            let probe = &self.probes[(iy * self.count_x + ix) as usize];
            let w = bilinear * probe.chebyshev_visibility(point);
            sum[0] += probe.irradiance.r * w;
            sum[1] += probe.irradiance.g * w;
            sum[2] += probe.irradiance.b * w;
            total += w;
        }
        if total <= 1e-6 {
            // The grid always holds at least one probe (counts are
            // asserted non-zero at construction).
            if let Some(nearest) = self.probes.iter().min_by(|a, b| {
                let da = (a.position - point).length_sq();
                let db = (b.position - point).length_sq();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            }) {
                return nearest.irradiance;
            }
            return Color::BLACK;
        }
        let inv = 1.0 / total;
        Color::rgb(sum[0] * inv, sum[1] * inv, sum[2] * inv)
    }
}

// ── 3D field ────────────────────────────────────────────────────────────────

/// The result of a successful sphere trace through a 3D field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceHit3D {
    /// Distance travelled along the ray, in voxels.
    pub distance: f32,
    /// Position where the ray stopped, in voxel units.
    pub point: Vec3,
    /// Number of sphere-trace steps taken.
    pub steps: u32,
}

/// A 3D signed distance field built from a voxel occupancy grid.
///
/// The volumetric counterpart of [`JumpFloodSdf2D`], with the same
/// conventions and invariants. Occupancy is indexed
/// `((z * height + y) * width + x)`, `true` = solid.
pub struct JumpFloodSdf3D {
    width: u32,
    height: u32,
    depth: u32,
    signed: Vec<f32>,
    nearest_seed: Vec<i32>,
    hash: u64,
}

impl JumpFloodSdf3D {
    /// Creates an empty field of the given size (distances saturate at
    /// the grid diagonal until the first rebuild).
    ///
    /// # Panics
    ///
    /// Panics if a dimension is zero or the grid size overflows.
    #[must_use]
    pub fn new(width: u32, height: u32, depth: u32) -> Self {
        assert!(
            width > 0 && height > 0 && depth > 0,
            "field dimensions must be non-zero"
        );
        let size = usize::try_from(u64::from(width) * u64::from(height) * u64::from(depth))
            .expect("field dimensions overflow");
        let diag = ((width * width + height * height + depth * depth) as f32).sqrt();
        Self {
            width,
            height,
            depth,
            signed: vec![diag - 0.5; size],
            nearest_seed: vec![-1; size],
            hash: 0,
        }
    }

    /// Builds a field directly from an occupancy grid.
    ///
    /// # Panics
    ///
    /// Panics if a dimension is zero or the occupancy length mismatches.
    #[must_use]
    pub fn from_occupancy(width: u32, height: u32, depth: u32, occupancy: &[bool]) -> Self {
        let mut field = Self::new(width, height, depth);
        field.rebuild(occupancy);
        field
    }

    /// Width of the field in voxels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height of the field in voxels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Depth of the field in voxels.
    #[must_use]
    pub const fn depth(&self) -> u32 {
        self.depth
    }

    const fn index(&self, x: u32, y: u32, z: u32) -> usize {
        ((z * self.height + y) * self.width + x) as usize
    }

    /// Rebuilds the field from `occupancy`, reusing the existing buffers.
    ///
    /// # Panics
    ///
    /// Panics if `occupancy.len() != width * height * depth`.
    pub fn rebuild(&mut self, occupancy: &[bool]) {
        let expected = (self.width * self.height * self.depth) as usize;
        assert_eq!(
            occupancy.len(),
            expected,
            "occupancy length must equal width * height * depth"
        );
        self.hash = occupancy_hash(self.width, self.height, self.depth, occupancy);
        let seeds_empty: Vec<bool> = occupancy.iter().map(|c| !c).collect();
        let nearest_solid = jfa_nearest_3d(self.width, self.height, self.depth, occupancy);
        let nearest_empty = jfa_nearest_3d(self.width, self.height, self.depth, &seeds_empty);
        let diag = ((self.width * self.width + self.height * self.height + self.depth * self.depth)
            as f32)
            .sqrt();
        let dims = (self.width, self.height, self.depth);
        for (i, cell) in self.signed.iter_mut().enumerate() {
            let (x, y, z) = unravel3(i as u32, dims);
            *cell = if occupancy[i] {
                0.5 - dist_to_seed_3d((x, y, z), nearest_empty[i], dims, diag)
            } else {
                dist_to_seed_3d((x, y, z), nearest_solid[i], dims, diag) - 0.5
            };
        }
        self.nearest_seed = nearest_solid;
    }

    /// Rebuilds only if `occupancy` changed since the last build.
    /// Returns `true` if a rebuild happened.
    ///
    /// # Panics
    ///
    /// Panics if `occupancy.len() != width * height * depth`.
    pub fn rebuild_if_changed(&mut self, occupancy: &[bool]) -> bool {
        let expected = (self.width * self.height * self.depth) as usize;
        assert_eq!(
            occupancy.len(),
            expected,
            "occupancy length must equal width * height * depth"
        );
        if occupancy_hash(self.width, self.height, self.depth, occupancy) == self.hash
            && self.hash != 0
        {
            return false;
        }
        self.rebuild(occupancy);
        true
    }

    /// Signed distance at voxel `(x, y, z)`. Out-of-bounds coordinates
    /// clamp to the border.
    #[must_use]
    pub fn signed_distance(&self, x: u32, y: u32, z: u32) -> f32 {
        let idx = self.index(
            x.min(self.width - 1),
            y.min(self.height - 1),
            z.min(self.depth - 1),
        );
        self.signed[idx]
    }

    /// The whole signed-distance grid, in occupancy index order.
    #[must_use]
    pub fn distances(&self) -> &[f32] {
        &self.signed
    }

    /// The nearest solid voxel to `(x, y, z)`, or `None` if the grid
    /// has no solid voxels.
    #[must_use]
    pub fn nearest_seed(&self, x: u32, y: u32, z: u32) -> Option<(u32, u32, u32)> {
        let idx = self.index(
            x.min(self.width - 1),
            y.min(self.height - 1),
            z.min(self.depth - 1),
        );
        let seed = self.nearest_seed[idx];
        if seed < 0 {
            None
        } else {
            Some(unravel3(seed as u32, (self.width, self.height, self.depth)))
        }
    }

    /// Trilinearly interpolated signed distance at continuous position
    /// `p` (voxel units). Positions outside the grid clamp to the border.
    #[must_use]
    pub fn sample(&self, p: Vec3) -> f32 {
        let fx = p.x.clamp(0.0, (self.width - 1) as f32);
        let fy = p.y.clamp(0.0, (self.height - 1) as f32);
        let fz = p.z.clamp(0.0, (self.depth - 1) as f32);
        let x0 = fx.floor() as u32;
        let y0 = fy.floor() as u32;
        let z0 = fz.floor() as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let z1 = (z0 + 1).min(self.depth - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;
        let tz = fz - z0 as f32;
        let c000 = self.signed[self.index(x0, y0, z0)];
        let c100 = self.signed[self.index(x1, y0, z0)];
        let c010 = self.signed[self.index(x0, y1, z0)];
        let c110 = self.signed[self.index(x1, y1, z0)];
        let c001 = self.signed[self.index(x0, y0, z1)];
        let c101 = self.signed[self.index(x1, y0, z1)];
        let c011 = self.signed[self.index(x0, y1, z1)];
        let c111 = self.signed[self.index(x1, y1, z1)];
        let x00 = c000 + (c100 - c000) * tx;
        let x10 = c010 + (c110 - c010) * tx;
        let x01 = c001 + (c101 - c001) * tx;
        let x11 = c011 + (c111 - c011) * tx;
        let y0v = x00 + (x10 - x00) * ty;
        let y1v = x01 + (x11 - x01) * ty;
        y0v + (y1v - y0v) * tz
    }

    /// Approximate surface normal at `p` (normalized field gradient).
    /// Returns [`Vec3::ZERO`] where the field is locally flat.
    #[must_use]
    pub fn normal_at(&self, p: Vec3) -> Vec3 {
        let e = 0.5;
        let gx = self.sample(p + Vec3::new(e, 0.0, 0.0)) - self.sample(p - Vec3::new(e, 0.0, 0.0));
        let gy = self.sample(p + Vec3::new(0.0, e, 0.0)) - self.sample(p - Vec3::new(0.0, e, 0.0));
        let gz = self.sample(p + Vec3::new(0.0, 0.0, e)) - self.sample(p - Vec3::new(0.0, 0.0, e));
        normalize3(Vec3::new(gx, gy, gz))
    }

    /// Sphere-traces a ray against the 3D field. Same contract as
    /// [`JumpFloodSdf2D::trace`].
    #[must_use]
    pub fn trace(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Option<TraceHit3D> {
        if max_distance <= 0.0 || direction.length_sq() < 1e-12 {
            return None;
        }
        let dir = normalize3(direction);
        let mut t = 0.0f32;
        let mut steps = 0u32;
        loop {
            let p = origin + dir * t;
            let d = self.sample(p);
            steps += 1;
            if d <= TRACE_HIT_EPSILON {
                return Some(TraceHit3D {
                    distance: t,
                    point: p,
                    steps,
                });
            }
            t += d;
            if t > max_distance || steps >= MAX_TRACE_STEPS {
                return None;
            }
        }
    }

    /// Sphere-traced soft-shadow visibility in 3D; same estimator as
    /// [`JumpFloodSdf2D::shadow_visibility`].
    #[must_use]
    pub fn shadow_visibility(&self, point: Vec3, light: Vec3, sharpness: f32) -> f32 {
        let to_light = light - point;
        let dist = to_light.length();
        if dist < 1e-6 {
            return 1.0;
        }
        let dir = to_light * (1.0 / dist);
        let mut res = 1.0f32;
        let mut t = 0.0f32;
        for _ in 0..MAX_TRACE_STEPS {
            let p = point + dir * t;
            let d = self.sample(p);
            if d <= TRACE_HIT_EPSILON {
                return 0.0;
            }
            if t > 1e-4 {
                res = res.min(sharpness * d / t);
            }
            t += d;
            if t >= dist {
                break;
            }
        }
        res.clamp(0.0, 1.0)
    }
}

/// Splits a flat 3D index into `(x, y, z)` for `(width, height, depth)`.
const fn unravel3(index: u32, dims: (u32, u32, u32)) -> (u32, u32, u32) {
    let (w, h, _d) = dims;
    let x = index % w;
    let y = (index / w) % h;
    let z = index / (w * h);
    (x, y, z)
}

/// Squared distance from `(x, y, z)` to a seed flat index.
fn seed_dist2_3d(p: (i32, i32, i32), seed: i32, dims: (u32, u32, u32)) -> i64 {
    if seed < 0 {
        return i64::MAX;
    }
    let (sx, sy, sz) = unravel3(seed as u32, dims);
    let dx = i64::from(p.0 - sx as i32);
    let dy = i64::from(p.1 - sy as i32);
    let dz = i64::from(p.2 - sz as i32);
    dx * dx + dy * dy + dz * dz
}

fn dist_to_seed_3d(p: (u32, u32, u32), seed: i32, dims: (u32, u32, u32), fallback: f32) -> f32 {
    let d2 = seed_dist2_3d((p.0 as i32, p.1 as i32, p.2 as i32), seed, dims);
    if d2 == i64::MAX {
        fallback
    } else {
        (d2 as f32).sqrt()
    }
}

/// One JFA flood over a 3D grid (26-neighbourhood per pass).
fn jfa_nearest_3d(width: u32, height: u32, depth: u32, is_seed: &[bool]) -> Vec<i32> {
    let dims = (width, height, depth);
    let size = (width * height * depth) as usize;
    let mut cur: Vec<i32> = (0..size)
        .map(|i| if is_seed[i] { i as i32 } else { -1 })
        .collect();
    if cur.iter().all(|s| *s < 0) {
        return cur;
    }
    let mut next = cur.clone();
    for step in jfa_steps(width.max(height).max(depth)) {
        for z in 0..depth as i32 {
            for y in 0..height as i32 {
                for x in 0..width as i32 {
                    let idx = ((z * height as i32 + y) * width as i32 + x) as usize;
                    let mut best = cur[idx];
                    let mut best_d2 = seed_dist2_3d((x, y, z), best, dims);
                    for dz in [-step, 0, step] {
                        let nz = z + dz;
                        if nz < 0 || nz >= depth as i32 {
                            continue;
                        }
                        for dy in [-step, 0, step] {
                            let ny = y + dy;
                            if ny < 0 || ny >= height as i32 {
                                continue;
                            }
                            for dx in [-step, 0, step] {
                                let nx = x + dx;
                                if nx < 0 || nx >= width as i32 {
                                    continue;
                                }
                                let cand =
                                    cur[((nz * height as i32 + ny) * width as i32 + nx) as usize];
                                if cand < 0 {
                                    continue;
                                }
                                let d2 = seed_dist2_3d((x, y, z), cand, dims);
                                if d2 < best_d2 {
                                    best_d2 = d2;
                                    best = cand;
                                }
                            }
                        }
                    }
                    next[idx] = best;
                }
            }
        }
        std::mem::swap(&mut cur, &mut next);
    }
    cur
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Tiny deterministic LCG for property tests (no external crates).
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0 >> 33
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    /// Brute-force signed distance (same centre-to-centre convention
    /// and diagonal saturation as the field) for verification.
    fn brute_signed_2d(w: u32, h: u32, occ: &[bool], x: u32, y: u32) -> f32 {
        let diag = ((w * w + h * h) as f32).sqrt();
        let mut d_solid = diag;
        let mut d_empty = diag;
        for sy in 0..h {
            for sx in 0..w {
                let dx = x as f32 - sx as f32;
                let dy = y as f32 - sy as f32;
                let d = (dx * dx + dy * dy).sqrt();
                if occ[(sy * w + sx) as usize] {
                    d_solid = d_solid.min(d);
                } else {
                    d_empty = d_empty.min(d);
                }
            }
        }
        if occ[(y * w + x) as usize] {
            0.5 - d_empty
        } else {
            d_solid - 0.5
        }
    }

    fn block_occupancy(w: u32, h: u32, x0: u32, x1: u32, y0: u32, y1: u32) -> Vec<bool> {
        let mut occ = vec![false; (w * h) as usize];
        for y in y0..=y1 {
            for x in x0..=x1 {
                occ[(y * w + x) as usize] = true;
            }
        }
        occ
    }

    #[test]
    fn empty_grid_saturates_positive() {
        let occ = vec![false; 16 * 16];
        let field = JumpFloodSdf2D::from_occupancy(16, 16, &occ);
        let diag = (2.0f32 * 256.0).sqrt();
        for y in 0..16 {
            for x in 0..16 {
                assert!((field.signed_distance(x, y) - (diag - 0.5)).abs() < 1e-4);
                assert_eq!(field.nearest_seed(x, y), None);
            }
        }
    }

    #[test]
    fn full_grid_saturates_negative() {
        let occ = vec![true; 16 * 16];
        let field = JumpFloodSdf2D::from_occupancy(16, 16, &occ);
        let diag = (2.0f32 * 256.0).sqrt();
        for y in 0..16 {
            for x in 0..16 {
                assert!((field.signed_distance(x, y) - (0.5 - diag)).abs() < 1e-4);
                assert_eq!(field.nearest_seed(x, y), Some((x, y)));
            }
        }
    }

    #[test]
    fn single_seed_exact_distances() {
        let mut occ = vec![false; 16 * 16];
        occ[8 * 16 + 8] = true;
        let field = JumpFloodSdf2D::from_occupancy(16, 16, &occ);
        assert!((field.signed_distance(8, 8) - (-0.5)).abs() < 1e-5);
        assert!((field.signed_distance(9, 8) - 0.5).abs() < 1e-5);
        assert!((field.signed_distance(8, 5) - 2.5).abs() < 1e-5);
        let diag_expect = (50.0f32).sqrt() - 0.5; // (8,8) to (13,13): dx=dy=5
        assert!((field.signed_distance(13, 13) - diag_expect).abs() < 1e-4);
        assert_eq!(field.nearest_seed(0, 0), Some((8, 8)));
        assert_eq!(field.nearest_seed(15, 15), Some((8, 8)));
    }

    #[test]
    fn nearest_seed_picks_the_closer_seed() {
        let mut occ = vec![false; 20 * 4];
        occ[4] = true; // (4, 0)
        occ[16] = true; // (16, 0)
        let field = JumpFloodSdf2D::from_occupancy(20, 4, &occ);
        assert_eq!(field.nearest_seed(9, 0), Some((4, 0)));
        assert_eq!(field.nearest_seed(11, 0), Some((16, 0)));
    }

    #[test]
    fn block_matches_brute_force_everywhere() {
        let occ = block_occupancy(24, 18, 7, 14, 5, 11);
        let field = JumpFloodSdf2D::from_occupancy(24, 18, &occ);
        for y in 0..18 {
            for x in 0..24 {
                let expect = brute_signed_2d(24, 18, &occ, x, y);
                let got = field.signed_distance(x, y);
                assert!(
                    (got - expect).abs() < 1e-3,
                    "at ({x},{y}): jfa {got} vs brute {expect}"
                );
            }
        }
        // Centre of the block is 4 cells from the nearest empty cell.
        assert!((field.signed_distance(10, 8) - (-3.5)).abs() < 1e-4);
    }

    #[test]
    fn random_grids_match_brute_force_and_never_underestimate() {
        let mut rng = Lcg(0x5eed_1234);
        for trial in 0..10 {
            let (w, h) = (16u32, 16u32);
            let density = 5 + rng.below(30); // percent
            let mut occ = vec![false; (w * h) as usize];
            for cell in &mut occ {
                *cell = rng.below(100) < density;
            }
            if !occ.iter().any(|c| *c) || occ.iter().all(|c| *c) {
                continue;
            }
            let field = JumpFloodSdf2D::from_occupancy(w, h, &occ);
            for y in 0..h {
                for x in 0..w {
                    let expect = brute_signed_2d(w, h, &occ, x, y);
                    let got = field.signed_distance(x, y);
                    assert!(
                        (got - expect).abs() < 1e-3,
                        "trial {trial} at ({x},{y}): jfa {got} vs brute {expect}"
                    );
                    // The stored seed must be a real solid cell, and the
                    // distance to it must match the field value.
                    if let Some((sx, sy)) = field.nearest_seed(x, y) {
                        assert!(occ[(sy * w + sx) as usize], "seed must be solid");
                        if !occ[(y * w + x) as usize] {
                            let dx = x as f32 - sx as f32;
                            let dy = y as f32 - sy as f32;
                            let d = (dx * dx + dy * dy).sqrt() - 0.5;
                            assert!((d - got).abs() < 1e-3);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sample_interpolates_and_matches_grid_points() {
        let occ = block_occupancy(24, 18, 7, 14, 5, 11);
        let field = JumpFloodSdf2D::from_occupancy(24, 18, &occ);
        // At grid points, sample == grid value.
        assert!((field.sample(Vec2::new(3.0, 4.0)) - field.signed_distance(3, 4)).abs() < 1e-5);
        // Halfway between two grid points: the average of their values.
        let a = field.signed_distance(3, 4);
        let b = field.signed_distance(4, 4);
        let mid = field.sample(Vec2::new(3.5, 4.0));
        assert!((mid - (a + b) * 0.5).abs() < 1e-4);
        // The zero crossing on the block's left face sits at x = 6.5.
        let face = field.sample(Vec2::new(6.5, 8.0));
        assert!(face.abs() < 0.3, "face sample {face}");
    }

    #[test]
    fn normal_points_away_from_surface() {
        let occ = block_occupancy(24, 18, 7, 14, 5, 11);
        let field = JumpFloodSdf2D::from_occupancy(24, 18, &occ);
        // Left of the block, the normal points −x.
        let n = field.normal_at(Vec2::new(4.0, 8.0));
        assert!(n.x < -0.9, "normal {n:?}");
        // Above the block, it points −y… wait: above means smaller y,
        // surface below, so the gradient points toward −y.
        let n = field.normal_at(Vec2::new(10.0, 2.0));
        assert!(n.y < -0.9, "normal {n:?}");
        // Inside the block's right half, gradient still points away
        // from the centre: +x-ish.
        let n = field.normal_at(Vec2::new(13.0, 8.0));
        assert!(n.x > 0.9, "normal {n:?}");
    }

    #[test]
    fn trace_hits_block_face_and_misses_clear_rays() {
        let occ = block_occupancy(24, 18, 7, 14, 5, 11);
        let field = JumpFloodSdf2D::from_occupancy(24, 18, &occ);
        // Ray along y=8 from the left edge: face plane at x = 6.5.
        let hit = field
            .trace(Vec2::new(0.0, 8.0), Vec2::new(1.0, 0.0), 100.0)
            .expect("ray must hit the block");
        assert!(
            (hit.distance - 6.5).abs() < 0.25,
            "hit distance {}",
            hit.distance
        );
        assert!(hit.steps < 32, "sphere tracing should converge fast");
        // Ray above the block misses.
        assert!(
            field
                .trace(Vec2::new(0.0, 1.0), Vec2::new(1.0, 0.0), 100.0)
                .is_none()
        );
        // Ray pointing away misses.
        assert!(
            field
                .trace(Vec2::new(0.0, 8.0), Vec2::new(-1.0, 0.0), 100.0)
                .is_none()
        );
        // Non-normalized direction works the same.
        let hit2 = field
            .trace(Vec2::new(0.0, 8.0), Vec2::new(7.0, 0.0), 100.0)
            .expect("scaled direction still hits");
        assert!((hit2.distance - hit.distance).abs() < 1e-3);
        // Zero direction is rejected, not a panic.
        assert!(
            field
                .trace(Vec2::new(0.0, 8.0), Vec2::ZERO, 100.0)
                .is_none()
        );
    }

    #[test]
    fn trace_from_inside_hits_at_zero() {
        let occ = block_occupancy(24, 18, 7, 14, 5, 11);
        let field = JumpFloodSdf2D::from_occupancy(24, 18, &occ);
        let hit = field
            .trace(Vec2::new(10.0, 8.0), Vec2::new(1.0, 0.0), 100.0)
            .expect("inside origin is an immediate hit");
        assert_eq!(hit.distance, 0.0);
    }

    #[test]
    fn shadow_lit_occluded_and_penumbra() {
        // Block at x 14..=17, y 14..=17 in a 32×32 field; light to the left.
        let occ = block_occupancy(32, 32, 14, 17, 14, 17);
        let field = JumpFloodSdf2D::from_occupancy(32, 32, &occ);
        let light = Vec2::new(4.0, 16.0);
        // Point between light and block: fully lit.
        assert_eq!(
            field.shadow_visibility(Vec2::new(9.0, 16.0), light, 8.0),
            1.0
        );
        // Point directly behind the block: fully shadowed.
        assert_eq!(
            field.shadow_visibility(Vec2::new(26.0, 16.0), light, 8.0),
            0.0
        );
        // Shading point inside solid is dark.
        assert_eq!(
            field.shadow_visibility(Vec2::new(16.0, 16.0), light, 8.0),
            0.0
        );
        // A point whose light ray grazes the block's corner region:
        // partial penumbra, strictly between.
        let pen = field.shadow_visibility(Vec2::new(27.0, 21.5), light, 8.0);
        assert!(pen > 0.0 && pen < 1.0, "penumbra value {pen}");
    }

    #[test]
    fn rebuild_if_changed_only_rebuilds_on_change() {
        let occ_a = block_occupancy(16, 16, 4, 6, 4, 6);
        let occ_b = block_occupancy(16, 16, 9, 11, 9, 11);
        let mut field = JumpFloodSdf2D::new(16, 16);
        assert!(field.rebuild_if_changed(&occ_a));
        assert!(!field.rebuild_if_changed(&occ_a));
        let before = field.signed_distance(5, 5);
        assert!(before < 0.0);
        assert!(field.rebuild_if_changed(&occ_b));
        assert!(field.signed_distance(5, 5) > 0.0);
        assert!(field.signed_distance(10, 10) < 0.0);
    }

    #[test]
    fn occupancy_from_sdf_2d_rasterizes_circle() {
        let occ = occupancy_from_sdf_2d(32, 32, |p| {
            crate::sdf::circle_2d(p, Vec2::new(16.0, 16.0), 6.0)
        });
        assert!(occ[16 * 32 + 16], "centre is solid");
        assert!(!occ[0], "corner is empty");
        let count = occ.iter().filter(|c| **c).count();
        // π·6² ≈ 113 cells; rasterization at integer points is approximate.
        assert!(count > 90 && count < 140, "circle cell count {count}");
    }

    // ── Probes ──────────────────────────────────────────────────────────

    #[test]
    fn chebyshev_visibility_math() {
        let mut probe = IrradianceProbe::new(Vec2::ZERO);
        probe.mean_distance = 5.0;
        probe.mean_distance_sq = 25.0; // zero variance
        // Closer than the mean distance: fully visible.
        assert_eq!(probe.chebyshev_visibility(Vec2::new(3.0, 0.0)), 1.0);
        // Beyond with zero variance: fully hidden.
        assert_eq!(probe.chebyshev_visibility(Vec2::new(9.0, 0.0)), 0.0);
        // With variance, visibility decays smoothly and monotonically.
        probe.mean_distance_sq = 34.0; // variance 9
        let near = probe.chebyshev_visibility(Vec2::new(7.0, 0.0));
        let far = probe.chebyshev_visibility(Vec2::new(11.0, 0.0));
        assert!(near > far && far > 0.0, "near {near} far {far}");
        // var / (var + excess²) = 9 / (9 + 4) for excess 2.
        assert!((near - 9.0 / 13.0).abs() < 1e-5);
    }

    #[test]
    fn probes_see_only_sky_in_empty_field() {
        let occ = vec![false; 32 * 32];
        let field = JumpFloodSdf2D::from_occupancy(32, 32, &occ);
        let sky = Color::rgb(0.2, 0.3, 0.5);
        let mut grid = ProbeGrid2D::new(Vec2::ZERO, 8.0, 4, 4, 16);
        grid.update(&field, 40.0, sky, &|_, _| Color::WHITE);
        for probe in grid.probes() {
            assert!((probe.irradiance.r - sky.r).abs() < 1e-5);
            assert!((probe.irradiance.g - sky.g).abs() < 1e-5);
            assert!((probe.irradiance.b - sky.b).abs() < 1e-5);
            assert!((probe.mean_distance - 40.0).abs() < 1e-4);
        }
        let s = grid.sample(Vec2::new(13.0, 21.0));
        assert!((s.r - sky.r).abs() < 1e-4 && (s.b - sky.b).abs() < 1e-4);
    }

    #[test]
    fn probes_inside_room_see_walls() {
        // Room: solid border of a 32×32 grid, empty interior.
        let mut occ = vec![false; 32 * 32];
        for i in 0..32 {
            occ[i] = true;
            occ[31 * 32 + i] = true;
            occ[i * 32] = true;
            occ[i * 32 + 31] = true;
        }
        let field = JumpFloodSdf2D::from_occupancy(32, 32, &occ);
        let wall = Color::rgb(0.8, 0.6, 0.4);
        let mut grid = ProbeGrid2D::new(Vec2::new(8.0, 8.0), 8.0, 3, 3, 32);
        grid.update(&field, 64.0, Color::BLACK, &|_, _| wall);
        let centre = grid.probe(1, 1).expect("centre probe exists");
        // Every ray from the centre hits a wall — no sky leaks.
        assert!((centre.irradiance.r - wall.r).abs() < 1e-4);
        assert!((centre.irradiance.g - wall.g).abs() < 1e-4);
        // Mean hit distance is room-scale, well under the 64 max.
        assert!(
            centre.mean_distance > 5.0 && centre.mean_distance < 20.0,
            "mean distance {}",
            centre.mean_distance
        );
        let s = grid.sample(Vec2::new(16.0, 16.0));
        assert!((s.r - wall.r).abs() < 1e-3);
    }

    #[test]
    fn sample_downweights_occluded_probe() {
        // Two probes side by side. Probe A sits behind a close occluder
        // (tiny mean distance, zero variance); probe B sees far. A point
        // midway, beyond A's mean distance, must take B's colour.
        let mut grid = ProbeGrid2D::new(Vec2::ZERO, 10.0, 2, 1, 8);
        grid.probes[0].irradiance = Color::RED;
        grid.probes[0].mean_distance = 1.0;
        grid.probes[0].mean_distance_sq = 1.0;
        grid.probes[1].irradiance = Color::BLUE;
        grid.probes[1].mean_distance = 100.0;
        grid.probes[1].mean_distance_sq = 10_000.0;
        let s = grid.sample(Vec2::new(5.0, 0.0));
        assert!(s.b > 0.99 && s.r < 0.01, "sample {s:?}");
        // And at A's own position, A is fully visible and wins.
        let s = grid.sample(Vec2::new(0.0, 0.0));
        assert!(s.r > 0.99, "sample at A {s:?}");
    }

    #[test]
    fn sample_blends_neighbouring_probes() {
        let mut grid = ProbeGrid2D::new(Vec2::ZERO, 10.0, 2, 1, 8);
        for (i, probe) in grid.probes.iter_mut().enumerate() {
            probe.mean_distance = 100.0;
            probe.mean_distance_sq = 10_000.0;
            if i == 1 {
                probe.irradiance = Color::WHITE;
            }
        }
        let mid = grid.sample(Vec2::new(5.0, 0.0));
        assert!((mid.r - 0.5).abs() < 1e-4, "midpoint blend {mid:?}");
    }

    // ── 3D ──────────────────────────────────────────────────────────────

    fn block_occupancy_3d(n: u32, lo: u32, hi: u32) -> Vec<bool> {
        let mut occ = vec![false; (n * n * n) as usize];
        for z in lo..=hi {
            for y in lo..=hi {
                for x in lo..=hi {
                    occ[((z * n + y) * n + x) as usize] = true;
                }
            }
        }
        occ
    }

    #[test]
    fn sdf3d_single_voxel_exact() {
        let n = 12u32;
        let mut occ = vec![false; (n * n * n) as usize];
        occ[((6 * n + 6) * n + 6) as usize] = true;
        let field = JumpFloodSdf3D::from_occupancy(n, n, n, &occ);
        assert!((field.signed_distance(6, 6, 6) - (-0.5)).abs() < 1e-5);
        assert!((field.signed_distance(7, 6, 6) - 0.5).abs() < 1e-5);
        let body = (3.0f32).sqrt() - 0.5; // body-diagonal neighbour
        assert!((field.signed_distance(7, 7, 7) - body).abs() < 1e-4);
        assert_eq!(field.nearest_seed(0, 0, 0), Some((6, 6, 6)));
    }

    #[test]
    fn sdf3d_random_matches_brute_force() {
        let mut rng = Lcg(0x3d_5eed);
        let n = 10u32;
        let mut occ = vec![false; (n * n * n) as usize];
        for cell in &mut occ {
            *cell = rng.below(100) < 15;
        }
        let field = JumpFloodSdf3D::from_occupancy(n, n, n, &occ);
        let diag = ((3 * n * n) as f32).sqrt();
        for z in 0..n {
            for y in 0..n {
                for x in 0..n {
                    // Brute force with the same convention.
                    let mut d_solid = diag;
                    let mut d_empty = diag;
                    for sz in 0..n {
                        for sy in 0..n {
                            for sx in 0..n {
                                let dx = x as f32 - sx as f32;
                                let dy = y as f32 - sy as f32;
                                let dz = z as f32 - sz as f32;
                                let d = (dx * dx + dy * dy + dz * dz).sqrt();
                                if occ[((sz * n + sy) * n + sx) as usize] {
                                    d_solid = d_solid.min(d);
                                } else {
                                    d_empty = d_empty.min(d);
                                }
                            }
                        }
                    }
                    let expect = if occ[((z * n + y) * n + x) as usize] {
                        0.5 - d_empty
                    } else {
                        d_solid - 0.5
                    };
                    let got = field.signed_distance(x, y, z);
                    assert!(
                        (got - expect).abs() < 1e-3,
                        "at ({x},{y},{z}): jfa {got} vs brute {expect}"
                    );
                }
            }
        }
    }

    #[test]
    fn sdf3d_sample_trace_and_shadow() {
        let n = 20u32;
        let occ = block_occupancy_3d(n, 8, 11);
        let field = JumpFloodSdf3D::from_occupancy(n, n, n, &occ);
        // Trilinear sample at a grid point equals the grid value.
        assert!(
            (field.sample(Vec3::new(3.0, 10.0, 10.0)) - field.signed_distance(3, 10, 10)).abs()
                < 1e-5
        );
        // Ray along +x at y=z=10 hits the block face at x = 7.5.
        let hit = field
            .trace(Vec3::new(0.0, 10.0, 10.0), Vec3::new(1.0, 0.0, 0.0), 100.0)
            .expect("ray must hit the block");
        assert!((hit.distance - 7.5).abs() < 0.25, "hit {}", hit.distance);
        // Ray that passes beside the block misses.
        assert!(
            field
                .trace(Vec3::new(0.0, 2.0, 2.0), Vec3::new(1.0, 0.0, 0.0), 100.0)
                .is_none()
        );
        // Shadow: lit in front, dark behind, along the same axis.
        let light = Vec3::new(2.0, 10.0, 10.0);
        assert_eq!(
            field.shadow_visibility(Vec3::new(5.0, 10.0, 10.0), light, 8.0),
            1.0
        );
        assert_eq!(
            field.shadow_visibility(Vec3::new(16.0, 10.0, 10.0), light, 8.0),
            0.0
        );
        // Normal on the −x side of the block points −x.
        let normal = field.normal_at(Vec3::new(5.0, 10.0, 10.0));
        assert!(normal.x < -0.9, "normal {normal:?}");
    }

    #[test]
    fn sdf3d_rebuild_if_changed() {
        let n = 8u32;
        let occ_a = block_occupancy_3d(n, 2, 3);
        let occ_b = block_occupancy_3d(n, 5, 6);
        let mut field = JumpFloodSdf3D::new(n, n, n);
        assert!(field.rebuild_if_changed(&occ_a));
        assert!(!field.rebuild_if_changed(&occ_a));
        assert!(field.signed_distance(2, 2, 2) < 0.0);
        assert!(field.rebuild_if_changed(&occ_b));
        assert!(field.signed_distance(2, 2, 2) > 0.0);
        assert!(field.signed_distance(5, 5, 5) < 0.0);
    }

    #[test]
    fn occupancy_from_sdf_3d_rasterizes_sphere() {
        let occ = occupancy_from_sdf_3d(16, 16, 16, |p| {
            crate::sdf::sphere_3d(p, Vec3::new(8.0, 8.0, 8.0), 4.0)
        });
        assert!(occ[((8 * 16 + 8) * 16 + 8) as usize], "centre is solid");
        assert!(!occ[0], "corner is empty");
        let count = occ.iter().filter(|c| **c).count();
        // (4/3)π·4³ ≈ 268 voxels, approximately.
        assert!(count > 200 && count < 340, "sphere voxel count {count}");
    }
}
