#![allow(
    clippy::float_cmp,
    clippy::imprecise_flops,
    clippy::too_long_first_doc_paragraph
)]
//! Analytic narrow-band ray intersection for baked SDF grids.
//!
//! A baked signed-distance field — an analytic scene sampled onto a
//! regular grid, then reconstructed by trilinear interpolation — is the
//! standard way to render shapes that have no closed-form distance
//! (scanned models, CSG soups, destructible terrain). The standard way
//! to *ray-trace* one is sphere tracing: sample the field, step by the
//! sampled distance, repeat. That stalls exactly where it matters:
//! near the surface the steps shrink toward the hit tolerance, and a
//! ray grazing the surface can burn hundreds of samples travelling a
//! few cells.
//!
//! Pujol & Chica (CEIG 2024, "Rendering piecewise approximations of
//! SDFs through analytic intersections") observe that the field inside
//! one cell is not arbitrary. Trilinear interpolation is degree-1 in
//! each axis separately, so restricted to a ray `p(t) = ro + t·rd` the
//! interpolant is a **cubic polynomial in `t`**:
//!
//! ```text
//! u(t) = aₓ + bₓt,  v(t) = a_y + b_y t,  w(t) = a_z + b_z t
//! f(t) = Σ cᵢⱼₖ · Xᵢ(u) Yⱼ(v) Zₖ(w)        (X₀ = 1−u, X₁ = u, …)
//!      = C₀ + C₁t + C₂t² + C₃t³
//! ```
//!
//! So instead of marching *within* a cell, walk the grid cell by cell
//! (a 3D DDA), and in each cell either
//!
//! * **skip it outright** when all eight corners lie above the narrow
//!   band: the interpolant is a convex combination of the corners
//!   inside the cell, so it cannot dip below their minimum — no root,
//!   no samples, one comparison; or
//! * **solve it analytically** when the corners straddle the narrow
//!   band: split the cell's `t`-interval at the cubic's critical
//!   points (the derivative is a quadratic), and the first
//!   sign-changing segment brackets exactly one root, which bisection
//!   pins down. A root that merely *touches* (a tangent ray) sits at a
//!   critical point and is found there.
//!
//! The narrow band (half-width [`TraceParams::narrow_band`]) decides
//! how many cells pay for a cubic solve; it cannot change the answer,
//! because the cell containing a crossing always has corners on both
//! sides of zero and therefore always lies inside any band ≥ 0.
//!
//! # Invariants (the spec these functions are tested against)
//!
//! 1. The interpolant inside a cell lies within `[min, max]` of the
//!    cell's eight corners, so a cell with `min > band` contains no
//!    surface and skipping it is exact, not heuristic.
//! 2. [`SdfGrid::cell_ray_polynomial`] reproduces [`SdfGrid::sample`]
//!    along the ray everywhere inside its cell (the tests check this
//!    against the independent trilinear sampler).
//! 3. A root reported by [`SdfGrid::intersect`] satisfies
//!    `|sample(hit_point)|` at the level of trilinear accuracy: for
//!    fields that are linear along the ray (a baked plane, a sphere
//!    crossed on a grid axis) the hit distance is exact to float
//!    precision; for curved fields it matches the true surface to
//!    within the grid's interpolation error, never the march tolerance
//!    of a sphere trace.
//! 4. [`SdfGrid::intersect`] visits each cell at most once (DDA), so
//!    `cells_visited ≤ nx + ny + nz` for any ray, however tangent.
//! 5. [`SdfGrid::sphere_trace`] is the honest baseline: same grid, same
//!    hit convention, classic sphere marching — kept so callers (and
//!    the demo) can measure the analytic path against it.
//!
//! # Conventions
//!
//! * Node `(i, j, k)` sits at `origin + spacing · (i, j, k)`; values
//!   are row-major with `x` fastest: `values[i + nx·(j + ny·k)]`.
//! * Negative is inside, as everywhere in this crate's SDF code.
//! * Ray directions need not be normalised; hit distances `t` are in
//!   world units along the *normalised* direction. A ray starting
//!   inside the solid reports a hit at its grid-entry distance (0 when
//!   it starts inside the grid) — the surface is at or behind it.
//! * [`SdfGrid::sample`] clamps to the border outside the grid, while
//!   the ray queries treat leaving the grid as a miss.

use crate::math::Vec3;

/// A regular grid of baked signed-distance samples.
#[derive(Debug, Clone, PartialEq)]
pub struct SdfGrid {
    nx: usize,
    ny: usize,
    nz: usize,
    origin: Vec3,
    spacing: f32,
    values: Vec<f32>,
}

impl SdfGrid {
    /// Create a grid from baked samples.
    ///
    /// # Errors
    ///
    /// Returns `Err` when any dimension is below 2, `spacing` is not
    /// positive and finite, `origin` has a non-finite component,
    /// `values.len() != nx·ny·nz`, or any value is non-finite.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::analytic_sdf_grid::SdfGrid;
    /// use abrash_core::math::Vec3;
    ///
    /// // The plane x = 0 on a 2×2×2 grid: values are just the x of each node.
    /// let grid = SdfGrid::new(2, 2, 2, Vec3::new(-1.0, -1.0, -1.0), 2.0,
    ///     vec![-1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0]).unwrap();
    /// assert_eq!(grid.sample(Vec3::new(0.0, 0.3, -0.2)), 0.0);
    /// ```
    pub fn new(
        nx: usize,
        ny: usize,
        nz: usize,
        origin: Vec3,
        spacing: f32,
        values: Vec<f32>,
    ) -> Result<Self, &'static str> {
        if nx < 2 || ny < 2 || nz < 2 {
            return Err("grid dimensions must be at least 2 per axis");
        }
        if !spacing.is_finite() || spacing <= 0.0 {
            return Err("spacing must be positive and finite");
        }
        if !origin.x.is_finite() || !origin.y.is_finite() || !origin.z.is_finite() {
            return Err("origin must be finite");
        }
        if values.len() != nx * ny * nz {
            return Err("values length must equal nx*ny*nz");
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err("values must all be finite");
        }
        Ok(Self {
            nx,
            ny,
            nz,
            origin,
            spacing,
            values,
        })
    }

    /// Bake a grid by sampling `sdf` at every node.
    ///
    /// # Errors
    ///
    /// Same conditions as [`SdfGrid::new`]; a non-finite value returned
    /// by `sdf` at any node is an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::analytic_sdf_grid::SdfGrid;
    /// use abrash_core::math::Vec3;
    ///
    /// // A sphere of radius 2 centred at the origin.
    /// let grid = SdfGrid::bake(17, 17, 17, Vec3::splat(-4.0), 0.5,
    ///     |p| p.length() - 2.0).unwrap();
    /// let hit = grid.intersect(Vec3::new(-6.0, 0.0, 0.0), Vec3::X,
    ///     &Default::default()).hit.unwrap();
    /// // Down the axis the field is linear, so the hit is exact.
    /// assert!((hit.t - 4.0).abs() < 1e-3, "t = {}", hit.t);
    /// ```
    pub fn bake(
        nx: usize,
        ny: usize,
        nz: usize,
        origin: Vec3,
        spacing: f32,
        sdf: impl Fn(Vec3) -> f32,
    ) -> Result<Self, &'static str> {
        if nx < 2 || ny < 2 || nz < 2 {
            return Err("grid dimensions must be at least 2 per axis");
        }
        if !spacing.is_finite() || spacing <= 0.0 {
            return Err("spacing must be positive and finite");
        }
        let mut values = Vec::with_capacity(nx * ny * nz);
        for k in 0..nz {
            for j in 0..ny {
                for i in 0..nx {
                    let p = Vec3::new(
                        origin.x + spacing * i as f32,
                        origin.y + spacing * j as f32,
                        origin.z + spacing * k as f32,
                    );
                    values.push(sdf(p));
                }
            }
        }
        Self::new(nx, ny, nz, origin, spacing, values)
    }

    /// Grid dimensions in nodes per axis: `(nx, ny, nz)`.
    #[must_use]
    pub const fn dims(&self) -> (usize, usize, usize) {
        (self.nx, self.ny, self.nz)
    }

    /// Distance between neighbouring nodes.
    #[must_use]
    pub const fn spacing(&self) -> f32 {
        self.spacing
    }

    /// World position of node `(0, 0, 0)`.
    #[must_use]
    pub const fn origin(&self) -> Vec3 {
        self.origin
    }

    /// The baked samples, row-major with `x` fastest.
    #[must_use]
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    /// Bytes held by the sample array — the whole memory cost of the
    /// baked field (the analytic method needs no auxiliary structure).
    #[must_use]
    pub const fn memory_bytes(&self) -> usize {
        self.values.len() * std::mem::size_of::<f32>()
    }

    /// World position of node `(i, j, k)`.
    #[must_use]
    pub fn node_position(&self, i: usize, j: usize, k: usize) -> Vec3 {
        Vec3::new(
            self.origin.x + self.spacing * i as f32,
            self.origin.y + self.spacing * j as f32,
            self.origin.z + self.spacing * k as f32,
        )
    }

    /// World-space bounds of the grid: `(min, max)` node positions.
    #[must_use]
    pub fn bounds(&self) -> (Vec3, Vec3) {
        (
            self.origin,
            self.node_position(self.nx - 1, self.ny - 1, self.nz - 1),
        )
    }

    fn value_at(&self, i: usize, j: usize, k: usize) -> f32 {
        self.values[i + self.nx * (j + self.ny * k)]
    }

    /// The eight corner values of cell `(i, j, k)`, in `(x, y, z)` bit
    /// order: index `x + 2y + 4z` picks the high node on that axis.
    ///
    /// Returns `None` when the cell is out of range.
    #[must_use]
    pub fn cell_corners(&self, i: usize, j: usize, k: usize) -> Option<[f32; 8]> {
        if i + 1 >= self.nx || j + 1 >= self.ny || k + 1 >= self.nz {
            return None;
        }
        Some(self.corners(i, j, k))
    }

    /// Corner values of a cell the caller has already bounds-checked.
    fn corners(&self, i: usize, j: usize, k: usize) -> [f32; 8] {
        let mut c = [0.0f32; 8];
        for (idx, corner) in c.iter_mut().enumerate() {
            *corner = self.value_at(i + (idx & 1), j + ((idx >> 1) & 1), k + ((idx >> 2) & 1));
        }
        c
    }

    /// Trilinear sample of the field at `p`, clamped to the border
    /// outside the grid.
    #[must_use]
    pub fn sample(&self, p: Vec3) -> f32 {
        let g = Vec3::new(
            ((p.x - self.origin.x) / self.spacing).clamp(0.0, (self.nx - 1) as f32),
            ((p.y - self.origin.y) / self.spacing).clamp(0.0, (self.ny - 1) as f32),
            ((p.z - self.origin.z) / self.spacing).clamp(0.0, (self.nz - 1) as f32),
        );
        let (i, u) = cell_and_frac(g.x, self.nx);
        let (j, v) = cell_and_frac(g.y, self.ny);
        let (k, w) = cell_and_frac(g.z, self.nz);
        let c = self.corners(i, j, k);
        trilinear(&c, u, v, w)
    }

    /// Analytic gradient of the trilinear interpolant at `p` — the
    /// derivative of the same polynomial the ray solver uses, *not* a
    /// finite difference of the sampler, so shading normals and hit
    /// normals agree with the intersected surface. (The gradient is
    /// per-cell: it can jump across a cell face, as can the true
    /// gradient of a trilinear field.)
    #[must_use]
    pub fn sample_gradient(&self, p: Vec3) -> Vec3 {
        let g = Vec3::new(
            ((p.x - self.origin.x) / self.spacing).clamp(0.0, (self.nx - 1) as f32),
            ((p.y - self.origin.y) / self.spacing).clamp(0.0, (self.ny - 1) as f32),
            ((p.z - self.origin.z) / self.spacing).clamp(0.0, (self.nz - 1) as f32),
        );
        let (i, u) = cell_and_frac(g.x, self.nx);
        let (j, v) = cell_and_frac(g.y, self.ny);
        let (k, w) = cell_and_frac(g.z, self.nz);
        let c = self.corners(i, j, k);
        // ∂/∂u, ∂/∂v, ∂/∂w of the trilinear form, then ÷ spacing.
        let du = lerp(
            lerp(c[1] - c[0], c[3] - c[2], v),
            lerp(c[5] - c[4], c[7] - c[6], v),
            w,
        );
        let dv = lerp(
            lerp(c[2] - c[0], c[3] - c[1], u),
            lerp(c[6] - c[4], c[7] - c[5], u),
            w,
        );
        let dw = lerp(
            lerp(c[4] - c[0], c[5] - c[1], u),
            lerp(c[6] - c[2], c[7] - c[3], u),
            v,
        );
        Vec3::new(du, dv, dw) * (1.0 / self.spacing)
    }

    /// Surface normal at `p`: the normalised analytic gradient, or
    /// zero where the gradient vanishes (deep inside a constant cell).
    #[must_use]
    pub fn normal(&self, p: Vec3) -> Vec3 {
        self.sample_gradient(p).normalize_or_zero()
    }

    /// The cubic coefficients `[C₀, C₁, C₂, C₃]` of the trilinear
    /// interpolant restricted to the ray `ro + t·rd` inside cell
    /// `(i, j, k)`, with `t` in world units along `rd` exactly as
    /// passed (normalise `rd` first if `t` should be a distance).
    ///
    /// Returns `None` when the cell is out of range.
    #[must_use]
    pub fn cell_ray_polynomial(
        &self,
        i: usize,
        j: usize,
        k: usize,
        ro: Vec3,
        rd: Vec3,
    ) -> Option<[f32; 4]> {
        let c = self.cell_corners(i, j, k)?;
        let node = self.node_position(i, j, k);
        // Local cell coordinates along the ray: axis(t) = a + b·t.
        let a = [
            (ro.x - node.x) / self.spacing,
            (ro.y - node.y) / self.spacing,
            (ro.z - node.z) / self.spacing,
        ];
        let b = [
            rd.x / self.spacing,
            rd.y / self.spacing,
            rd.z / self.spacing,
        ];
        // Each trilinear factor is linear in t: for corner bit 0 the
        // factor is (1 − axis) = [(1−a), −b]; for bit 1 it is [a, b].
        // The product of the three axis factors is a cubic; accumulate
        // corner-weighted convolutions of the three linear factors.
        let mut coeff = [0.0f32; 4];
        for idx in 0..8 {
            let fx = axis_factor(idx & 1, a[0], b[0]);
            let fy = axis_factor((idx >> 1) & 1, a[1], b[1]);
            let fz = axis_factor((idx >> 2) & 1, a[2], b[2]);
            let prod = [
                fx[0] * fy[0] * fz[0],
                fx[1] * fy[0] * fz[0] + fx[0] * fy[1] * fz[0] + fx[0] * fy[0] * fz[1],
                fx[1] * fy[1] * fz[0] + fx[1] * fy[0] * fz[1] + fx[0] * fy[1] * fz[1],
                fx[1] * fy[1] * fz[1],
            ];
            for n in 0..4 {
                coeff[n] += c[idx] * prod[n];
            }
        }
        Some(coeff)
    }

    /// Intersect a ray with the zero level set by DDA cell traversal
    /// plus per-cell analytic cubic solves in the narrow band.
    ///
    /// See the module docs for the algorithm and its invariants.
    #[must_use]
    pub fn intersect(&self, ro: Vec3, rd: Vec3, params: &TraceParams) -> TraceOutcome {
        let params = params.sanitised(self.spacing);
        let mut out = TraceOutcome::default();
        let Some(dir) = unit_direction(ro, rd) else {
            return out;
        };
        let Some((t_enter, t_exit)) = self.ray_grid_interval(ro, dir) else {
            return out;
        };
        let t_exit = t_exit.min(params.max_distance);
        if t_enter > t_exit {
            return out;
        }
        let start = ro + dir * t_enter;
        let g = [
            (start.x - self.origin.x) / self.spacing,
            (start.y - self.origin.y) / self.spacing,
            (start.z - self.origin.z) / self.spacing,
        ];
        let dims = [self.nx, self.ny, self.nz];
        let dg = [
            dir.x / self.spacing,
            dir.y / self.spacing,
            dir.z / self.spacing,
        ];
        let mut cell = [0usize; 3];
        let mut t_max = [f32::INFINITY; 3];
        let mut t_delta = [f32::INFINITY; 3];
        let mut step = [0i32; 3];
        for axis in 0..3 {
            let (c, first_plane_t, delta) = dda_axis(g[axis], dg[axis], dims[axis], t_enter);
            cell[axis] = c;
            t_max[axis] = first_plane_t;
            t_delta[axis] = delta;
            step[axis] = if dg[axis] > 0.0 {
                1
            } else if dg[axis] < 0.0 {
                -1
            } else {
                0
            };
        }

        let mut t_cell_start = t_enter;
        // A DDA visits at most nx+ny+nz cells; the bound also guards
        // against any float pathology in the plane distances.
        let max_cells = self.nx + self.ny + self.nz + 3;
        while out.cells_visited < max_cells {
            let corners = self.corners(cell[0], cell[1], cell[2]);
            let cmin = corners.iter().copied().fold(f32::INFINITY, f32::min);
            let cmax = corners.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let t_cell_end = t_max[0].min(t_max[1]).min(t_max[2]).min(t_exit);
            out.cells_visited += 1;

            if cmin > params.narrow_band {
                // Invariant 1: no surface in this cell. Skip it.
                out.cells_skipped += 1;
            } else if cmax < 0.0 {
                // Wholly inside the solid: the entry point of this
                // cell is already past the surface. On the first cell
                // that is the documented start-inside convention; later
                // it can only be reached across a face whose crossing
                // the previous cell's solve owned, so report the face.
                out.hit = Some(self.make_hit(ro, dir, t_cell_start));
                return out;
            } else {
                out.cells_solved += 1;
                let coeff = self
                    .cell_ray_polynomial(cell[0], cell[1], cell[2], ro, dir)
                    .unwrap_or([f32::INFINITY; 4]);
                let f_start = eval_cubic(&coeff, t_cell_start);
                if f_start < 0.0 {
                    // Started (or crossed into) the solid inside this
                    // cell without a bracketed entry root: the surface
                    // is at or behind the cell entry.
                    out.hit = Some(self.make_hit(ro, dir, t_cell_start));
                    return out;
                }
                if let Some(t) = first_root_in_interval(&coeff, t_cell_start, t_cell_end) {
                    out.hit = Some(self.make_hit(ro, dir, t));
                    return out;
                }
            }

            if t_cell_end >= t_exit {
                break;
            }
            // Advance across the nearest pending grid plane.
            let axis = if t_max[0] <= t_max[1] && t_max[0] <= t_max[2] {
                0
            } else if t_max[1] <= t_max[2] {
                1
            } else {
                2
            };
            t_cell_start = t_max[axis];
            let next = cell[axis] as i32 + step[axis];
            if next < 0 || next as usize + 1 >= dims[axis] {
                break;
            }
            cell[axis] = next as usize;
            t_max[axis] += t_delta[axis];
        }
        out
    }

    /// The baseline: classic sphere tracing of the same trilinear
    /// field, kept for comparison. Steps by the sampled distance
    /// (clamped below so a near-tangent ray still progresses) and
    /// reports a hit when a sample falls within `hit_epsilon` of the
    /// surface — which is exactly the tolerance a sphere trace's
    /// answer carries, and the analytic path does not.
    #[must_use]
    pub fn sphere_trace(&self, ro: Vec3, rd: Vec3, params: &TraceParams) -> TraceOutcome {
        let params = params.sanitised(self.spacing);
        let mut out = TraceOutcome::default();
        let Some(dir) = unit_direction(ro, rd) else {
            return out;
        };
        let Some((t_enter, t_exit)) = self.ray_grid_interval(ro, dir) else {
            return out;
        };
        let t_exit = t_exit.min(params.max_distance);
        if t_enter > t_exit {
            return out;
        }
        let min_step = self.spacing * 1e-3;
        let mut t = t_enter;
        // Generous cap: the grazing-ray stall is the phenomenon under
        // study, so the baseline must be allowed to exhibit it.
        for _ in 0..100_000 {
            let p = ro + dir * t;
            let d = self.sample(p);
            out.field_samples += 1;
            if d <= params.hit_epsilon {
                out.hit = Some(self.make_hit(ro, dir, t));
                return out;
            }
            t += d.max(min_step);
            if t > t_exit {
                return out;
            }
        }
        out
    }

    fn make_hit(&self, ro: Vec3, dir: Vec3, t: f32) -> RayHit {
        let point = ro + dir * t;
        let mut normal = self.normal(point);
        if normal == Vec3::ZERO {
            normal = dir * -1.0;
        }
        RayHit { t, point, normal }
    }

    /// Slab intersection of the ray with the grid's node bounds, as
    /// `(t_enter, t_exit)` in world units along the normalised `dir`
    /// (`t_enter` clamped to ≥ 0), or `None` when the ray misses the
    /// grid entirely.
    fn ray_grid_interval(&self, ro: Vec3, dir: Vec3) -> Option<(f32, f32)> {
        let (bmin, bmax) = self.bounds();
        let ro_a = [ro.x, ro.y, ro.z];
        let dir_a = [dir.x, dir.y, dir.z];
        let min_a = [bmin.x, bmin.y, bmin.z];
        let max_a = [bmax.x, bmax.y, bmax.z];
        let mut t0 = f32::NEG_INFINITY;
        let mut t1 = f32::INFINITY;
        for axis in 0..3 {
            if dir_a[axis] == 0.0 {
                if ro_a[axis] < min_a[axis] || ro_a[axis] > max_a[axis] {
                    return None;
                }
            } else {
                let inv = 1.0 / dir_a[axis];
                let mut near = (min_a[axis] - ro_a[axis]) * inv;
                let mut far = (max_a[axis] - ro_a[axis]) * inv;
                if near > far {
                    std::mem::swap(&mut near, &mut far);
                }
                t0 = t0.max(near);
                t1 = t1.min(far);
                if t0 > t1 {
                    return None;
                }
            }
        }
        if t1 < 0.0 {
            return None;
        }
        Some((t0.max(0.0), t1))
    }
}

/// Parameters for [`SdfGrid::intersect`] and [`SdfGrid::sphere_trace`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceParams {
    /// Furthest hit distance accepted, in world units.
    pub max_distance: f32,
    /// Half-width of the narrow band around zero, in field units:
    /// cells whose corners all lie above it are skipped without a
    /// cubic solve. Affects only how many cells are *solved*, never
    /// the result (see module invariant). The default (`∞`) is a
    /// sentinel resolved to `1.5 × spacing` for the grid in use.
    pub narrow_band: f32,
    /// Sphere-trace hit tolerance (analytic path: unused except as
    /// documented; the cubic root needs no epsilon). Default: `1e-3`.
    pub hit_epsilon: f32,
}

impl Default for TraceParams {
    fn default() -> Self {
        Self {
            max_distance: 1_000.0,
            narrow_band: f32::INFINITY, // resolved against spacing in sanitised()
            hit_epsilon: 1e-3,
        }
    }
}

impl TraceParams {
    fn sanitised(self, spacing: f32) -> Self {
        Self {
            max_distance: if self.max_distance.is_finite() && self.max_distance > 0.0 {
                self.max_distance
            } else {
                1_000.0
            },
            narrow_band: if !self.narrow_band.is_finite() || self.narrow_band < 0.0 {
                // The Default value (∞) is a sentinel: resolve it
                // against the grid's spacing.
                1.5 * spacing
            } else {
                self.narrow_band
            },
            hit_epsilon: if self.hit_epsilon.is_finite() && self.hit_epsilon > 0.0 {
                self.hit_epsilon
            } else {
                1e-3
            },
        }
    }
}

/// A ray–surface intersection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    /// Distance along the normalised ray direction.
    pub t: f32,
    /// World-space hit point.
    pub point: Vec3,
    /// Surface normal at the hit (analytic gradient, normalised).
    pub normal: Vec3,
}

/// Outcome of a ray query: the hit, if any, plus the work counters
/// that make the analytic-versus-marching comparison measurable.
/// Every counter is exact for the path that produced it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TraceOutcome {
    /// The first intersection, if the ray hit the surface.
    pub hit: Option<RayHit>,
    /// Cells examined by the analytic DDA (0 for sphere tracing).
    pub cells_visited: usize,
    /// Cells skipped by the narrow-band corner test with no cubic
    /// solve and no field sample.
    pub cells_skipped: usize,
    /// Cells in the narrow band whose cubic was solved.
    pub cells_solved: usize,
    /// Field samples taken by sphere tracing (0 for the analytic
    /// path — it reads corners, never marches).
    pub field_samples: usize,
}

/// Evaluate the cubic `c[0] + c[1]·t + c[2]·t² + c[3]·t³` (Horner).
///
/// # Examples
///
/// ```
/// use abrash_core::analytic_sdf_grid::eval_cubic;
///
/// // (t − 1)(t − 2) = t² − 3t + 2, as a cubic with a zero leading term.
/// assert_eq!(eval_cubic(&[2.0, -3.0, 1.0, 0.0], 1.5), -0.25);
/// ```
#[must_use]
pub fn eval_cubic(c: &[f32; 4], t: f32) -> f32 {
    ((c[3] * t + c[2]) * t + c[1]) * t + c[0]
}

/// The first root of the cubic `c` in `[t0, t1]`, if one exists.
///
/// The interval is split at the cubic's critical points (roots of the
/// quadratic derivative), making the cubic monotonic on each segment;
/// a segment whose endpoints straddle zero brackets exactly one root,
/// found by bisection, and a critical point or endpoint that *is*
/// zero is itself the root (the tangent case). Roots are returned in
/// increasing order — the first is the visible surface.
///
/// # Examples
///
/// ```
/// use abrash_core::analytic_sdf_grid::first_root_in_interval;
///
/// // (t − 0.2)(t − 0.5)(t − 0.8) = t³ − 1.5t² + 0.66t − 0.08.
/// let c = [-0.08, 0.66, -1.5, 1.0];
/// let r = first_root_in_interval(&c, 0.0, 1.0).unwrap();
/// assert!((r - 0.2).abs() < 1e-5, "r = {r}");
/// assert!(first_root_in_interval(&c, 0.85, 1.0).is_none());
/// ```
#[must_use]
pub fn first_root_in_interval(c: &[f32; 4], t0: f32, t1: f32) -> Option<f32> {
    if !(t0 <= t1) || !t0.is_finite() || !t1.is_finite() {
        return None;
    }
    let f0 = eval_cubic(c, t0);
    if f0 == 0.0 {
        return Some(t0);
    }
    // Critical points of 3c₃t² + 2c₂t + c₁ inside (t0, t1).
    let mut cuts: Vec<f32> = vec![t0];
    let (a, b, cc) = (3.0 * c[3], 2.0 * c[2], c[1]);
    if a != 0.0 {
        let disc = b * b - 4.0 * a * cc;
        if disc >= 0.0 {
            let sq = disc.sqrt();
            for r in [(-b - sq) / (2.0 * a), (-b + sq) / (2.0 * a)] {
                if r > t0 && r < t1 {
                    cuts.push(r);
                }
            }
        }
    } else if b != 0.0 {
        let r = -cc / b;
        if r > t0 && r < t1 {
            cuts.push(r);
        }
    }
    cuts.push(t1);
    cuts.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));

    let mut prev_t = cuts[0];
    let mut prev_f = f0;
    for &t in &cuts[1..] {
        let f = eval_cubic(c, t);
        if f == 0.0 {
            return Some(t);
        }
        if (prev_f < 0.0) != (f < 0.0) {
            // Monotonic segment with a sign change: one root, bisect.
            let (mut lo, mut hi) = (prev_t, t);
            let mut flo = prev_f;
            for _ in 0..60 {
                let mid = f32::midpoint(lo, hi);
                if mid == lo || mid == hi {
                    break;
                }
                let fm = eval_cubic(c, mid);
                if fm == 0.0 {
                    return Some(mid);
                }
                if (fm < 0.0) == (flo < 0.0) {
                    lo = mid;
                    flo = fm;
                } else {
                    hi = mid;
                }
            }
            return Some(f32::midpoint(lo, hi));
        }
        prev_t = t;
        prev_f = f;
    }
    None
}

// ── helpers ─────────────────────────────────────────────────────────

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn trilinear(c: &[f32; 8], u: f32, v: f32, w: f32) -> f32 {
    let x00 = lerp(c[0], c[1], u);
    let x10 = lerp(c[2], c[3], u);
    let x01 = lerp(c[4], c[5], u);
    let x11 = lerp(c[6], c[7], u);
    lerp(lerp(x00, x10, v), lerp(x01, x11, v), w)
}

/// Cell index and local fraction for a clamped grid coordinate.
fn cell_and_frac(g: f32, n: usize) -> (usize, f32) {
    let i = (g.floor() as usize).min(n - 2);
    (i, g - i as f32)
}

/// Linear-in-`t` trilinear factor for one axis and one corner bit.
fn axis_factor(bit: usize, a: f32, b: f32) -> [f32; 2] {
    if bit == 0 { [1.0 - a, -b] } else { [a, b] }
}

fn unit_direction(ro: Vec3, rd: Vec3) -> Option<Vec3> {
    if !ro.x.is_finite()
        || !ro.y.is_finite()
        || !ro.z.is_finite()
        || !rd.x.is_finite()
        || !rd.y.is_finite()
        || !rd.z.is_finite()
    {
        return None;
    }
    let len = rd.length();
    if len == 0.0 || !len.is_finite() {
        return None;
    }
    Some(rd * (1.0 / len))
}

/// Per-axis DDA initialisation: given the entry point's grid
/// coordinate `g`, the grid-space direction `dg` (per world unit of
/// `t`), and the node count, return `(cell, t_of_first_plane,
/// t_between_planes)`. Times are absolute (they include `t_enter`).
fn dda_axis(g: f32, dg: f32, n: usize, t_enter: f32) -> (usize, f32, f32) {
    if dg == 0.0 {
        let cell = (g.floor() as i32).clamp(0, n as i32 - 2) as usize;
        return (cell, f32::INFINITY, f32::INFINITY);
    }
    let delta = 1.0 / dg.abs();
    let floor_g = g.floor();
    let frac = g - floor_g;
    if dg > 0.0 {
        let cell = (floor_g as i32).clamp(0, n as i32 - 2) as usize;
        let dist = if frac == 0.0 {
            delta
        } else {
            (1.0 - frac) * delta
        };
        (cell, t_enter + dist, delta)
    } else {
        // Moving negative: on an exact plane the ray enters the cell
        // below the plane, whose far plane is one full step away.
        let (cell_i, dist) = if frac == 0.0 {
            (floor_g as i32 - 1, delta)
        } else {
            (floor_g as i32, frac * delta)
        };
        let cell = cell_i.clamp(0, n as i32 - 2) as usize;
        (cell, t_enter + dist, delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-3;

    fn sphere_grid() -> SdfGrid {
        SdfGrid::bake(17, 17, 17, Vec3::splat(-4.0), 0.5, |p| p.length() - 2.0)
            .expect("sphere grid bakes")
    }

    fn plane_grid() -> SdfGrid {
        SdfGrid::bake(9, 9, 9, Vec3::splat(-2.0), 0.5, |p| p.x).expect("plane grid bakes")
    }

    // ── construction / sampling ─────────────────────────────────────

    #[test]
    fn new_rejects_bad_inputs() {
        let o = Vec3::ZERO;
        assert!(SdfGrid::new(1, 2, 2, o, 1.0, vec![0.0; 4]).is_err());
        assert!(SdfGrid::new(2, 2, 2, o, 0.0, vec![0.0; 8]).is_err());
        assert!(SdfGrid::new(2, 2, 2, o, -1.0, vec![0.0; 8]).is_err());
        assert!(SdfGrid::new(2, 2, 2, o, f32::NAN, vec![0.0; 8]).is_err());
        assert!(SdfGrid::new(2, 2, 2, o, 1.0, vec![0.0; 7]).is_err());
        assert!(SdfGrid::new(2, 2, 2, o, 1.0, vec![f32::INFINITY; 8]).is_err());
        assert!(SdfGrid::new(2, 2, 2, Vec3::new(f32::NAN, 0.0, 0.0), 1.0, vec![0.0; 8]).is_err());
        assert!(SdfGrid::bake(4, 4, 4, o, 1.0, |_| f32::NAN).is_err());
    }

    #[test]
    fn sample_hits_nodes_and_interpolates_linear_fields_exactly() {
        let g = plane_grid();
        // Node values are reproduced exactly.
        assert_eq!(g.sample(g.node_position(3, 4, 5)), -0.5);
        // A plane is linear, so trilinear interpolation is exact anywhere.
        for p in [
            Vec3::new(0.3, 1.0, -1.0),
            Vec3::new(-1.7, 0.2, 0.9),
            Vec3::new(1.99, -2.0, 2.0),
        ] {
            assert!((g.sample(p) - p.x).abs() < 1e-6, "sample({p:?})");
        }
        // Outside clamps to the border value.
        assert_eq!(g.sample(Vec3::new(100.0, 0.0, 0.0)), 2.0);
        assert_eq!(g.sample(Vec3::new(-100.0, 0.0, 0.0)), -2.0);
    }

    #[test]
    fn sample_of_sphere_matches_analytic_at_nodes() {
        let g = sphere_grid();
        let node = g.node_position(8, 8, 12); // (0, 0, 2): on the sphere.
        assert!(g.sample(node).abs() < 1e-6);
        assert!(g.sample(Vec3::ZERO) < -1.9);
        assert!(g.sample(Vec3::new(4.0, 4.0, 4.0)) > 3.0);
    }

    #[test]
    fn gradient_of_plane_is_unit_x_everywhere() {
        let g = plane_grid();
        for p in [
            Vec3::ZERO,
            Vec3::new(0.7, -1.3, 0.4),
            Vec3::new(-2.0, 2.0, -2.0),
        ] {
            let grad = g.sample_gradient(p);
            assert!((grad - Vec3::X).length() < 1e-5, "grad {grad:?} at {p:?}");
            assert!((g.normal(p) - Vec3::X).length() < 1e-5);
        }
    }

    #[test]
    fn gradient_of_sphere_points_outward_on_axis() {
        let g = sphere_grid();
        // The per-cell trilinear gradient is exact in the axis
        // component on an axis line (the field along the axis is
        // linear there), but cross components from the neighbouring
        // nodes are real: the interpolant inside one cell is not
        // radially symmetric, so the normal is only approximately
        // radial. Pin both facts honestly.
        let grad = g.sample_gradient(Vec3::new(1.0, 0.0, 0.0));
        assert!((grad.x - 1.0).abs() < 1e-4, "grad {grad:?}");
        let n = g.normal(Vec3::new(1.0, 0.0, 0.0));
        assert!(n.x > 0.9, "normal {n:?}");
        let grad = g.sample_gradient(Vec3::new(0.0, 0.0, -1.5));
        assert!((grad.z + 1.0).abs() < 1e-4, "grad {grad:?}");
        let n = g.normal(Vec3::new(0.0, 0.0, -1.5));
        assert!(n.z < -0.9, "normal {n:?}");
    }

    #[test]
    fn memory_bytes_is_just_the_samples() {
        let g = sphere_grid();
        assert_eq!(g.memory_bytes(), 17 * 17 * 17 * 4);
    }

    // ── the cubic machinery ─────────────────────────────────────────

    #[test]
    fn cubic_root_finds_first_of_three() {
        let c = [-0.08, 0.66, -1.5, 1.0]; // roots 0.2, 0.5, 0.8
        let r = first_root_in_interval(&c, 0.0, 1.0).expect("root exists");
        assert!((r - 0.2).abs() < 1e-5);
        let r = first_root_in_interval(&c, 0.3, 1.0).expect("root exists");
        assert!((r - 0.5).abs() < 1e-5);
        assert!(first_root_in_interval(&c, 0.85, 1.0).is_none());
        assert!(first_root_in_interval(&c, 1.0, 0.0).is_none());
    }

    #[test]
    fn cubic_root_finds_tangent_double_root() {
        // (t − 0.4)²(t + 1) = t³ + 0.2t² − 0.64t + 0.16: touches zero
        // at t = 0.4 without a sign change. A double root's location
        // is ill-conditioned in f32 (perturbations move it by
        // ~sqrt(eps)), so the tolerance is 5e-4, not 1e-5.
        let c = [0.16, -0.64, 0.2, 1.0];
        let r = first_root_in_interval(&c, 0.0, 1.0).expect("tangent root exists");
        assert!((r - 0.4).abs() < 5e-4, "r = {r}");
    }

    #[test]
    fn cubic_root_handles_linear_and_constant() {
        // 2t − 1 as a cubic.
        assert!(
            (first_root_in_interval(&[-1.0, 2.0, 0.0, 0.0], 0.0, 1.0).expect("root") - 0.5).abs()
                < 1e-6
        );
        assert!(first_root_in_interval(&[3.0, 0.0, 0.0, 0.0], 0.0, 1.0).is_none());
        // Root exactly at the left endpoint.
        assert_eq!(
            first_root_in_interval(&[0.0, 1.0, 0.0, 0.0], 0.0, 1.0),
            Some(0.0)
        );
    }

    #[test]
    fn cell_polynomial_of_plane_is_the_ray_itself() {
        let g = plane_grid();
        let ro = Vec3::new(-3.0, 0.25, -0.5);
        let rd = Vec3::new(2.0, 0.5, 0.25);
        // Any cell: a globally linear field is reproduced exactly.
        let c = g
            .cell_ray_polynomial(4, 4, 4, ro, rd)
            .expect("cell in range");
        assert!((c[0] - ro.x).abs() < 1e-5, "c = {c:?}");
        assert!((c[1] - rd.x).abs() < 1e-5, "c = {c:?}");
        assert!(c[2].abs() < 1e-6 && c[3].abs() < 1e-6, "c = {c:?}");
        assert!(g.cell_ray_polynomial(8, 0, 0, ro, rd).is_none());
    }

    #[test]
    fn cell_polynomial_matches_sampler_inside_cell() {
        let g = sphere_grid();
        let ro = Vec3::new(-6.0, 0.7, -0.3);
        let dir = Vec3::X;
        // The ray is inside cell (0, 9, 7) for t in [2, 2.5]: check
        // the cubic against the independent trilinear sampler there.
        let cell = (0usize, 9usize, 7usize);
        let c = g
            .cell_ray_polynomial(cell.0, cell.1, cell.2, ro, dir)
            .expect("cell in range");
        for t in [2.0, 2.13, 2.25, 2.49] {
            let via_cubic = eval_cubic(&c, t);
            let via_sample = g.sample(ro + dir * t);
            assert!(
                (via_cubic - via_sample).abs() < 1e-4,
                "t = {t}: cubic {via_cubic} vs sample {via_sample}"
            );
        }
    }

    // ── analytic intersection ───────────────────────────────────────

    #[test]
    fn head_on_sphere_hit_is_exact_on_axis() {
        let g = sphere_grid();
        let out = g.intersect(Vec3::new(-6.0, 0.0, 0.0), Vec3::X, &TraceParams::default());
        let hit = out.hit.expect("ray hits the sphere");
        assert!((hit.t - 4.0).abs() < EPS, "t = {}", hit.t);
        assert!((hit.point - Vec3::new(-2.0, 0.0, 0.0)).length() < EPS);
        // Per-cell gradient: approximately radial (see the gradient
        // test for why it is not exact off a node row).
        assert!(hit.normal.x < -0.9, "normal {:?}", hit.normal);
        assert!(g.sample(hit.point).abs() < EPS);
        assert!(out.field_samples == 0, "analytic path never marches");
        assert!(out.cells_solved >= 1);
    }

    #[test]
    fn reverse_direction_hit_is_exact() {
        let g = sphere_grid();
        let out = g.intersect(
            Vec3::new(6.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            &TraceParams::default(),
        );
        let hit = out.hit.expect("ray hits");
        assert!((hit.t - 4.0).abs() < EPS, "t = {}", hit.t);
        assert!(hit.normal.x > 0.9, "normal {:?}", hit.normal);
    }

    #[test]
    fn non_unit_direction_still_reports_world_distance() {
        let g = sphere_grid();
        let out = g.intersect(
            Vec3::new(-6.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
            &TraceParams::default(),
        );
        assert!((out.hit.expect("hit").t - 4.0).abs() < EPS);
    }

    #[test]
    fn off_axis_sphere_hit_matches_true_surface() {
        let g = sphere_grid();
        // Ray at height (0.7, −0.3): true t = 6 − sqrt(4 − 0.49 − 0.09).
        let expected = 6.0 - (4.0f32 - 0.49 - 0.09).sqrt();
        let out = g.intersect(Vec3::new(-6.0, 0.7, -0.3), Vec3::X, &TraceParams::default());
        let hit = out.hit.expect("ray hits");
        assert!((hit.t - expected).abs() < 0.05, "t {} vs {expected}", hit.t);
        assert!(g.sample(hit.point).abs() < 0.02);
    }

    #[test]
    fn diagonal_sphere_hit_matches_true_surface() {
        let g = sphere_grid();
        let ro = Vec3::splat(-6.0);
        let dir = Vec3::ONE; // normalised internally, toward the centre
        let expected = ro.length() - 2.0;
        let out = g.intersect(ro, dir, &TraceParams::default());
        let hit = out.hit.expect("ray hits");
        assert!((hit.t - expected).abs() < 0.2, "t {} vs {expected}", hit.t);
    }

    #[test]
    fn slab_hit_is_exact_for_oblique_ray() {
        // A solid slab |x| < 0.5: the field |x| − 0.5 is piecewise
        // linear and reproduced exactly by trilinear interpolation
        // (the kink at x = 0 sits on a node plane). Entering the slab
        // from outside is a clean entry hit — unlike a half-space
        // plane, whose negative side is solid all the way to the grid
        // border and so reports the grid-entry hit by convention.
        let g = SdfGrid::bake(9, 9, 9, Vec3::splat(-2.0), 0.5, |p| p.x.abs() - 0.5)
            .expect("slab grid bakes");
        // From (−3, 1, 1) along (2, −1, 0.5): x reaches −0.5 after
        // Δx = 2.5, i.e. t = 2.5·|rd|/2 with |rd| = sqrt(5.25).
        let rd = Vec3::new(2.0, -1.0, 0.5);
        let expected = 2.5 * rd.length() / 2.0;
        let out = g.intersect(Vec3::new(-3.0, 1.0, 1.0), rd, &TraceParams::default());
        let hit = out.hit.expect("ray hits the slab");
        assert!((hit.t - expected).abs() < 1e-3, "t {} vs {expected}", hit.t);
        assert!((hit.point.x + 0.5).abs() < 1e-3);
        // The field is independent of y and z, so the normal is exact.
        assert!((hit.normal - Vec3::new(-1.0, 0.0, 0.0)).length() < 1e-3);
    }

    #[test]
    fn miss_rays_report_no_hit() {
        let g = sphere_grid();
        // Parallel ray passing beside the sphere.
        let out = g.intersect(Vec3::new(-6.0, 3.5, 3.5), Vec3::X, &TraceParams::default());
        assert!(out.hit.is_none());
        // Ray pointing away from the grid.
        let out = g.intersect(
            Vec3::new(-6.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            &TraceParams::default(),
        );
        assert!(out.hit.is_none());
        assert_eq!(out.cells_visited, 0);
        // Degenerate direction.
        let out = g.intersect(Vec3::ZERO, Vec3::ZERO, &TraceParams::default());
        assert!(out.hit.is_none());
    }

    #[test]
    fn far_ray_skips_every_cell_without_solving() {
        let g = sphere_grid();
        let out = g.intersect(Vec3::new(-6.0, 3.5, 3.5), Vec3::X, &TraceParams::default());
        assert!(out.hit.is_none());
        assert!(out.cells_visited > 0);
        assert_eq!(out.cells_skipped, out.cells_visited);
        assert_eq!(out.cells_solved, 0);
    }

    #[test]
    fn narrow_band_width_changes_work_not_answer() {
        let g = sphere_grid();
        let ro = Vec3::new(-6.0, 0.7, -0.3);
        let tight = TraceParams {
            narrow_band: 0.0,
            ..TraceParams::default()
        };
        let wide = TraceParams {
            narrow_band: 10.0,
            ..TraceParams::default()
        };
        let a = g.intersect(ro, Vec3::X, &tight).hit.expect("hit");
        let b = g.intersect(ro, Vec3::X, &wide).hit.expect("hit");
        assert!((a.t - b.t).abs() < 1e-4, "{} vs {}", a.t, b.t);
        let solved_tight = g.intersect(ro, Vec3::X, &tight).cells_solved;
        let solved_wide = g.intersect(ro, Vec3::X, &wide).cells_solved;
        assert!(solved_wide > solved_tight, "wide band solves more cells");
    }

    #[test]
    fn max_distance_cuts_off_hit() {
        let g = sphere_grid();
        let params = TraceParams {
            max_distance: 3.0,
            ..TraceParams::default()
        };
        let out = g.intersect(Vec3::new(-6.0, 0.0, 0.0), Vec3::X, &params);
        assert!(out.hit.is_none());
    }

    #[test]
    fn starting_inside_reports_entry_hit() {
        let g = sphere_grid();
        let out = g.intersect(Vec3::ZERO, Vec3::X, &TraceParams::default());
        let hit = out.hit.expect("inside start is a hit");
        assert_eq!(hit.t, 0.0);
    }

    #[test]
    fn grazing_ray_terminates_in_cells_not_samples() {
        let g = sphere_grid();
        // Just above the sphere: the true field stays ≈0.05–0.3 for
        // many cells — the classic sphere-tracing stall.
        let ro = Vec3::new(-6.0, 2.05, 0.0);
        let analytic = g.intersect(ro, Vec3::X, &TraceParams::default());
        assert!(
            analytic.hit.is_none(),
            "grazing ray misses: {:?}",
            analytic.hit
        );
        assert!(
            analytic.cells_visited <= 17 + 17 + 17,
            "DDA bound, visited {}",
            analytic.cells_visited
        );
        let marched = g.sphere_trace(ro, Vec3::X, &TraceParams::default());
        assert!(
            marched.field_samples > analytic.cells_visited,
            "marcher took {} samples vs {} cells",
            marched.field_samples,
            analytic.cells_visited
        );
    }

    // ── sphere-trace baseline ───────────────────────────────────────

    #[test]
    fn sphere_trace_agrees_on_axis_hit() {
        let g = sphere_grid();
        let out = g.sphere_trace(Vec3::new(-6.0, 0.0, 0.0), Vec3::X, &TraceParams::default());
        let hit = out.hit.expect("marcher hits");
        // Baseline accuracy is its epsilon, not float precision.
        assert!((hit.t - 4.0).abs() < 0.05, "t = {}", hit.t);
        assert!(out.field_samples > 0);
        assert_eq!(out.cells_visited, 0);
    }

    #[test]
    fn sphere_trace_misses_far_ray() {
        let g = sphere_grid();
        let out = g.sphere_trace(Vec3::new(-6.0, 3.5, 3.5), Vec3::X, &TraceParams::default());
        assert!(out.hit.is_none());
    }

    #[test]
    fn analytic_and_marched_agree_on_a_fan_of_rays() {
        let g = sphere_grid();
        // A fan of rays through the sphere: both methods must agree on
        // hit/miss, and on hit distance to marching tolerance.
        // Offsets chosen so no ray is near-tangent to the sphere
        // (every y²+z² is at least 0.75 away from r² = 4).
        for &y in &[-1.5f32, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5] {
            for &z in &[-1.5f32, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5] {
                let ro = Vec3::new(-6.0, y, z);
                let a = g.intersect(ro, Vec3::X, &TraceParams::default());
                let m = g.sphere_trace(ro, Vec3::X, &TraceParams::default());
                assert_eq!(a.hit.is_some(), m.hit.is_some(), "ray ({y}, {z})");
                if let (Some(ah), Some(mh)) = (a.hit, m.hit) {
                    assert!(
                        (ah.t - mh.t).abs() < 0.1,
                        "ray ({y}, {z}): analytic {} vs marched {}",
                        ah.t,
                        mh.t
                    );
                }
            }
        }
    }
}
