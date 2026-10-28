#![allow(
    clippy::float_cmp,
    clippy::imprecise_flops,
    clippy::suspicious_operation_groupings,
    clippy::too_long_first_doc_paragraph
)]
//! Software Gaussian splatting: project, sort, alpha-blend.
//!
//! Meshes are not the only way a scene arrives. Captured content — a room
//! scanned by a phone, a `NeRF` distilled into primitives — comes as a cloud
//! of *3D Gaussians* (Kerbl et al., "3D Gaussian Splatting", SIGGRAPH
//! 2023): each splat is an anisotropic ellipsoid with a colour and an
//! opacity. This module is the CPU reference version of that pipeline,
//! following the paper's three stages exactly:
//!
//! 1. **Project** ([`project_gaussian`]): each 3D covariance
//!    `Σ = R · diag(scale²) · Rᵀ` (rotation from a quaternion, per-axis
//!    scales) is moved into view space and pushed through the Jacobian of
//!    the perspective projection — the EWA splatting approximation
//!    (Zwicker et al. 2001) — giving a 2D screen-space covariance. A
//!    constant low-pass term (`0.3` added to the diagonal, as in the
//!    reference implementation) guarantees every projected splat covers
//!    at least a pixel, so nothing aliases away to a point.
//! 2. **Sort**: projected splats are ordered by view depth, nearest
//!    first. The sort is stable, so exact depth ties keep the input
//!    order and the image is deterministic for a given input.
//! 3. **Blend**: front-to-back premultiplied-alpha compositing with a
//!    per-pixel transmittance. A pixel whose transmittance falls to
//!    `1/255` or below stops accumulating — farther splats cannot change
//!    it by even one 8-bit step — which is the CPU-friendly half of the
//!    GPU tile rasteriser's early exit.
//!
//! # Conventions
//!
//! * World and view space follow [`crate::math`]: right-handed, camera
//!   looks down **-Z**, +Y is up. Screen space is pixels with **y down**
//!   (matching [`crate::framebuffer::Framebuffer`]); pixel centres are at
//!   half-integer coordinates.
//! * Colours are linear, as everywhere in [`crate::color::Color`]. The
//!   image stores straight (non-premultiplied) colours composited over
//!   [`SplatRenderParams::background`].
//! * `opacity` is clamped to `[0, 1]`; `scale` components are squared, so
//!   their sign does not matter, but a zero scale is fine — the low-pass
//!   filter still gives the splat a minimum footprint.
//!
//! # Invariants (the spec these functions are tested against)
//!
//! 1. A splat whose centre is behind the near plane, beyond the far
//!    plane, or non-finite in any component projects to `None`; so does
//!    one whose screen covariance is singular after the low-pass filter.
//! 2. At a projected splat's centre the alpha is exactly its (clamped)
//!    opacity; alpha falls off as `opacity · exp(-½ dᵀΣ⁻¹d)` and never
//!    exceeds the opacity.
//! 3. An opaque splat hides everything behind it at that pixel: the
//!    composited colour equals the opaque splat's colour whatever is
//!    submitted behind it.
//! 4. With distinct depths the rendered image does not depend on the
//!    input order — only on the depth sort. With tied depths the input
//!    order decides, deterministically.
//! 5. The footprint radius is `ceil(3σ)` of the largest covariance
//!    eigenvalue, clamped to `[1, max_radius]` — 3σ covers 98.9% of a
//!    1D Gaussian's mass along its widest axis, and beyond it the alpha
//!    is below the `1/255` blend cutoff anyway.
//!
//! # Scope
//!
//! This is the correctness-first reference: one global sort, no tiling,
//! no spherical-harmonics view-dependent colour (each splat has a single
//! RGB colour), no depth peeling. Those are throughput or fidelity
//! refinements that must not change the image this module produces.

use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, Vec2, Vec3};
use crate::quat::Quat;

/// Diagonal low-pass term added to every projected 2D covariance, in
/// squared pixels. Matches the reference 3DGS implementation: it stops
/// sub-pixel splats from aliasing (and from vanishing entirely).
pub const LOW_PASS_FILTER: f32 = 0.3;

/// Blends with alpha below this are skipped: they cannot change an
/// 8-bit output channel, and skipping them is what lets the
/// transmittance early-exit pay off.
pub const ALPHA_CUTOFF: f32 = 1.0 / 255.0;

/// Default footprint clamp, in pixels. A splat wider than this is drawn
/// with a truncated footprint (its centre still shades correctly); the
/// clamp exists so one enormous splat cannot make a frame cost
/// `O(screen²)` per splat without the caller opting in.
pub const DEFAULT_MAX_RADIUS: i32 = 512;

/// One 3D Gaussian splat: an anisotropic ellipsoid in world space.
///
/// The covariance is `R · diag(scale²) · Rᵀ` with `R` from `rotation`;
/// `scale` holds the standard deviations (not variances) along the
/// splat's local axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaussian {
    /// World-space centre.
    pub center: Vec3,
    /// Orientation of the ellipsoid's local axes.
    pub rotation: Quat,
    /// Standard deviations along the local axes (squared internally;
    /// sign is irrelevant, zero is allowed).
    pub scale: Vec3,
    /// Linear RGB colour (the alpha channel is ignored; use `opacity`).
    pub color: Color,
    /// Peak opacity at the splat's centre, clamped to `[0, 1]`.
    pub opacity: f32,
}

impl Gaussian {
    /// A general anisotropic splat. `opacity` is clamped to `[0, 1]`.
    #[must_use]
    pub const fn new(
        center: Vec3,
        rotation: Quat,
        scale: Vec3,
        color: Color,
        opacity: f32,
    ) -> Self {
        Self {
            center,
            rotation,
            scale,
            color,
            opacity: opacity.clamp(0.0, 1.0),
        }
    }

    /// A spherical splat with standard deviation `radius` on every axis.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::color::Color;
    /// use abrash_core::gaussian_splat::Gaussian;
    /// use abrash_core::math::Vec3;
    ///
    /// let g = Gaussian::isotropic(Vec3::new(0.0, 0.0, -6.0), 0.5, Color::RED, 0.9);
    /// assert_eq!(g.scale, Vec3::splat(0.5));
    /// assert_eq!(g.opacity, 0.9);
    /// ```
    #[must_use]
    pub const fn isotropic(center: Vec3, radius: f32, color: Color, opacity: f32) -> Self {
        Self::new(
            center,
            Quat::identity(),
            Vec3::splat(radius),
            color,
            opacity,
        )
    }

    /// The world-space covariance `R · diag(scale²) · Rᵀ`, as a row-major
    /// symmetric 3×3 matrix. Built from the images of the basis vectors,
    /// so no matrix convention can sneak in: column `j` of `R` is
    /// `rotation * e_j`, and `Σ = Σ_j scale_j² · (R e_j)(R e_j)ᵀ`.
    fn covariance(&self) -> [[f32; 3]; 3] {
        let q = if self.rotation.length_sq() > 1e-12 {
            self.rotation.normalize()
        } else {
            Quat::identity()
        };
        let axes = [
            q.rotate_vec3(Vec3::X),
            q.rotate_vec3(Vec3::Y),
            q.rotate_vec3(Vec3::Z),
        ];
        let vars = [self.scale.x, self.scale.y, self.scale.z].map(|s| s * s);
        let mut cov = [[0.0_f32; 3]; 3];
        for (axis, var) in axes.iter().zip(vars) {
            let a = [axis.x, axis.y, axis.z];
            for (i, ci) in cov.iter_mut().enumerate() {
                for (j, c) in ci.iter_mut().enumerate() {
                    *c += var * a[i] * a[j];
                }
            }
        }
        cov
    }
}

/// A pinhole camera for splat projection: a view matrix plus the
/// projection in pixel units.
///
/// Screen position of a view-space point `(x, y, z)` with depth
/// `d = -z` is `(center_x + focal_x · x/d, center_y - focal_y · y/d)` —
/// the minus sign flips view-space +Y (up) into screen-space +Y (down).
#[derive(Debug, Clone, Copy)]
pub struct SplatCamera {
    /// World-to-view transform (rotation + translation, affine).
    pub view: Mat4,
    /// Horizontal focal length in pixels.
    pub focal_x: f32,
    /// Vertical focal length in pixels.
    pub focal_y: f32,
    /// Principal point x in pixels (usually `width / 2`).
    pub center_x: f32,
    /// Principal point y in pixels (usually `height / 2`).
    pub center_y: f32,
    /// Near depth (view-space distance); centres closer than this cull.
    pub near: f32,
    /// Far depth; centres farther than this cull.
    pub far: f32,
}

impl SplatCamera {
    /// A camera from explicit pixel-unit projection parameters.
    #[must_use]
    pub const fn new(
        view: Mat4,
        focal_x: f32,
        focal_y: f32,
        center_x: f32,
        center_y: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            view,
            focal_x,
            focal_y,
            center_x,
            center_y,
            near,
            far,
        }
    }

    /// A camera from a vertical field of view: square pixels, principal
    /// point at the image centre, `focal = height / (2 · tan(fov/2))`.
    ///
    /// # Examples
    ///
    /// ```
    /// use abrash_core::gaussian_splat::SplatCamera;
    /// use abrash_core::math::Mat4;
    /// use std::f32::consts::FRAC_PI_2;
    ///
    /// // 90° vertical fov on a 100x100 image: focal length is 50 px.
    /// let cam = SplatCamera::from_fov_y(Mat4::identity(), 100, 100, FRAC_PI_2, 0.1, 100.0);
    /// assert!((cam.focal_x - 50.0).abs() < 1e-3);
    /// assert!((cam.center_x - 50.0).abs() < 1e-6);
    /// ```
    #[must_use]
    pub fn from_fov_y(
        view: Mat4,
        width: u32,
        height: u32,
        fov_y_radians: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let focal = height as f32 / (2.0 * (fov_y_radians * 0.5).tan());
        Self::new(
            view,
            focal,
            focal,
            width as f32 * 0.5,
            height as f32 * 0.5,
            near,
            far,
        )
    }
}

/// A splat after projection: everything the blend pass needs, nothing it
/// does not. Screen-space, y down, pixel centres at half-integers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectedSplat {
    /// Screen-space centre in pixels.
    pub center: Vec2,
    /// View-space depth (`-z` in view space); smaller is nearer.
    pub depth: f32,
    /// Screen covariance `[xx, xy, yy]` (after the low-pass filter).
    pub cov: [f32; 3],
    /// Inverse screen covariance `[xx, xy, yy]`.
    pub inv_cov: [f32; 3],
    /// Footprint radius in pixels: pixels whose centres are farther
    /// than this from `center` are never evaluated.
    pub radius: i32,
    /// Linear RGB colour.
    pub color: Color,
    /// Peak alpha (the splat's clamped opacity).
    pub opacity: f32,
    /// Index of the source [`Gaussian`] in the input slice.
    pub source_index: usize,
}

impl ProjectedSplat {
    /// Alpha at a pixel whose centre is offset `(dx, dy)` from the
    /// splat centre: `opacity · exp(-½ · dᵀΣ⁻¹d)`, clamped to
    /// `[0, opacity]` against float noise in the quadratic form.
    #[must_use]
    pub fn alpha_at_offset(&self, dx: f32, dy: f32) -> f32 {
        let [a, b, c] = self.inv_cov;
        let power = -0.5 * (a * dx * dx + 2.0 * b * dx * dy + c * dy * dy);
        if power >= 0.0 {
            return self.opacity;
        }
        (self.opacity * power.exp()).clamp(0.0, self.opacity)
    }

    /// Alpha at an absolute screen position.
    #[must_use]
    pub fn alpha_at(&self, point: Vec2) -> f32 {
        self.alpha_at_offset(point.x - self.center.x, point.y - self.center.y)
    }
}

/// Symmetric 3×3 quadratic form `vᵀ · Σ · v`.
fn quad_form(cov: &[[f32; 3]; 3], v: [f32; 3]) -> f32 {
    let r = [
        cov[0][0] * v[0] + cov[0][1] * v[1] + cov[0][2] * v[2],
        cov[1][0] * v[0] + cov[1][1] * v[1] + cov[1][2] * v[2],
        cov[2][0] * v[0] + cov[2][1] * v[1] + cov[2][2] * v[2],
    ];
    v[0] * r[0] + v[1] * r[1] + v[2] * r[2]
}

/// Symmetric 3×3 bilinear form `aᵀ · Σ · b`.
fn bilinear_form(cov: &[[f32; 3]; 3], a: [f32; 3], b: [f32; 3]) -> f32 {
    let r = [
        cov[0][0] * b[0] + cov[0][1] * b[1] + cov[0][2] * b[2],
        cov[1][0] * b[0] + cov[1][1] * b[1] + cov[1][2] * b[2],
        cov[2][0] * b[0] + cov[2][1] * b[1] + cov[2][2] * b[2],
    ];
    a[0] * r[0] + a[1] * r[1] + a[2] * r[2]
}

/// Project one Gaussian to screen space, or `None` when it cannot
/// contribute: centre outside `[near, far]`, any non-finite input, or a
/// screen covariance that stays singular after the low-pass filter.
///
/// The covariance path is basis-vector based throughout: the view
/// covariance is `Σ_view = Σ_jk Σ[j][k] · (W e_j)(W e_k)ᵀ` with `W` the
/// view rotation applied via `transform_vector`, and the screen
/// covariance is `Σ₂ = J Σ_view Jᵀ` evaluated as quadratic/bilinear
/// forms in the Jacobian's two rows — so the result cannot depend on a
/// row/column-major reading of any matrix.
#[must_use]
pub fn project_gaussian(
    gaussian: &Gaussian,
    camera: &SplatCamera,
    max_radius: i32,
) -> Option<ProjectedSplat> {
    let c = gaussian.center;
    let s = gaussian.scale;
    if ![c.x, c.y, c.z, s.x, s.y, s.z, gaussian.opacity]
        .iter()
        .all(|v| v.is_finite())
        || ![gaussian.color.r, gaussian.color.g, gaussian.color.b]
            .iter()
            .all(|v| v.is_finite())
    {
        return None;
    }

    let (view_pos, _w) = camera.view.transform_point(c);
    let depth = -view_pos.z;
    if !depth.is_finite() || depth < camera.near || depth > camera.far {
        return None;
    }

    // View-space covariance.
    let world_cov = gaussian.covariance();
    let cols = [
        camera.view.transform_vector(Vec3::X),
        camera.view.transform_vector(Vec3::Y),
        camera.view.transform_vector(Vec3::Z),
    ];
    let col = |v: Vec3| [v.x, v.y, v.z];
    let mut view_cov = [[0.0_f32; 3]; 3];
    for j in 0..3 {
        for k in 0..3 {
            let w = world_cov[j][k];
            if w == 0.0 {
                continue;
            }
            let (a, b) = (col(cols[j]), col(cols[k]));
            for (i, ci) in view_cov.iter_mut().enumerate() {
                for (m, cell) in ci.iter_mut().enumerate() {
                    *cell += w * a[i] * b[m];
                }
            }
        }
    }

    // Jacobian rows for (u, v) with d = depth:
    //   u = cx + fx·x/d  →  (fx/d, 0, fx·x/d²)
    //   v = cy - fy·y/d  →  (0, -fy/d, -fy·y/d²)
    let d2 = depth * depth;
    let row_u = [
        camera.focal_x / depth,
        0.0,
        camera.focal_x * view_pos.x / d2,
    ];
    let row_v = [
        0.0,
        -camera.focal_y / depth,
        -camera.focal_y * view_pos.y / d2,
    ];
    let mut cov_xx = quad_form(&view_cov, row_u);
    let cov_xy = bilinear_form(&view_cov, row_u, row_v);
    let mut cov_yy = quad_form(&view_cov, row_v);
    cov_xx += LOW_PASS_FILTER;
    cov_yy += LOW_PASS_FILTER;

    let det = cov_xx * cov_yy - cov_xy * cov_xy;
    if !det.is_finite() || det <= 0.0 {
        return None;
    }
    let inv_cov = [cov_yy / det, -cov_xy / det, cov_xx / det];

    // Largest eigenvalue of the 2×2 covariance, for the 3σ radius.
    let mid = f32::midpoint(cov_xx, cov_yy);
    let disc = (0.25 * (cov_xx - cov_yy) * (cov_xx - cov_yy) + cov_xy * cov_xy).sqrt();
    let lambda_max = mid + disc;
    if !lambda_max.is_finite() || lambda_max <= 0.0 {
        return None;
    }
    let radius = (3.0 * lambda_max.sqrt()).ceil() as i32;

    Some(ProjectedSplat {
        center: Vec2::new(
            camera.center_x + camera.focal_x * view_pos.x / depth,
            camera.center_y - camera.focal_y * view_pos.y / depth,
        ),
        depth,
        cov: [cov_xx, cov_xy, cov_yy],
        inv_cov,
        radius: radius.clamp(1, max_radius.max(1)),
        color: gaussian.color,
        opacity: gaussian.opacity.clamp(0.0, 1.0),
        source_index: 0,
    })
}

/// Knobs for [`render_gaussians`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplatRenderParams {
    /// Colour behind everything; the image is the splats composited
    /// over this. An opaque background (the default) makes the output
    /// alpha 1 everywhere.
    pub background: Color,
    /// Footprint radius clamp in pixels (see [`DEFAULT_MAX_RADIUS`]).
    pub max_radius: i32,
    /// Transmittance at or below which a pixel stops accumulating.
    pub transmittance_cutoff: f32,
}

impl Default for SplatRenderParams {
    fn default() -> Self {
        Self {
            background: Color::BLACK,
            max_radius: DEFAULT_MAX_RADIUS,
            transmittance_cutoff: ALPHA_CUTOFF,
        }
    }
}

/// Counters from one [`render_gaussians`] call.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SplatRenderStats {
    /// Gaussians in the input slice.
    pub splats_input: usize,
    /// Gaussians that projected successfully and entered the sort.
    pub splats_projected: usize,
    /// Gaussians culled by projection (frustum, non-finite, singular).
    pub splats_culled: usize,
    /// Pixel/splat pairs whose ellipse was actually evaluated.
    pub fragments_evaluated: usize,
    /// Evaluations that passed the alpha cutoff and blended.
    pub fragments_blended: usize,
    /// Evaluations skipped because the pixel's transmittance had
    /// already reached the cutoff (an opaque splat in front).
    pub fragments_skipped_saturated: usize,
    /// Pixels whose final colour differs from the pure background
    /// because at least one splat blended into them.
    pub pixels_painted: usize,
}

/// The rendered image: straight-alpha linear colours, row-major.
#[derive(Debug, Clone, PartialEq)]
pub struct SplatImage {
    width: u32,
    height: u32,
    pixels: Vec<Color>,
}

impl SplatImage {
    /// Image width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Image height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The colour at `(x, y)`, or `None` outside the image.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.pixels[(y * self.width + x) as usize])
    }

    /// All pixels, row-major, `width * height` entries.
    #[must_use]
    pub fn as_slice(&self) -> &[Color] {
        &self.pixels
    }

    /// Write the image into a framebuffer (sRGB-encoded via
    /// [`Color::to_argb_u32`]). Mismatched sizes write the overlap.
    pub fn write_to_framebuffer(&self, fb: &mut Framebuffer) {
        let w = self.width.min(fb.width());
        let h = self.height.min(fb.height());
        for y in 0..h {
            for x in 0..w {
                let c = self.pixels[(y * self.width + x) as usize];
                fb.set_pixel(x as i32, y as i32, c.to_argb_u32());
            }
        }
    }
}

/// Render a splat cloud to a new image of `width` × `height`.
///
/// Pipeline: project every splat, stable-sort the survivors by depth
/// (nearest first; ties keep input order), then composite front-to-back
/// with per-pixel transmittance and the early exit described in the
/// module docs.
///
/// # Errors
///
/// Returns an error for the same dimension limits as
/// [`Framebuffer::new`].
///
/// # Examples
///
/// ```
/// use abrash_core::color::Color;
/// use abrash_core::gaussian_splat::{
///     render_gaussians, Gaussian, SplatCamera, SplatRenderParams,
/// };
/// use abrash_core::math::{Mat4, Vec3};
/// use std::f32::consts::FRAC_PI_2;
///
/// let cam = SplatCamera::from_fov_y(Mat4::identity(), 16, 16, FRAC_PI_2, 0.1, 100.0);
/// let splats = [Gaussian::isotropic(
///     Vec3::new(0.0, 0.0, -4.0),
///     0.5,
///     Color::RED,
///     1.0,
/// )];
/// let (img, stats) = render_gaussians(&splats, &cam, &SplatRenderParams::default(), 16, 16)
///     .unwrap();
/// assert_eq!(stats.splats_projected, 1);
/// // The splat sits on the image centre: strong red there, none in the corner.
/// assert!(img.pixel(8, 8).unwrap().r > 0.5);
/// assert_eq!(img.pixel(0, 0).unwrap(), Color::BLACK);
/// ```
pub fn render_gaussians(
    splats: &[Gaussian],
    camera: &SplatCamera,
    params: &SplatRenderParams,
    width: u32,
    height: u32,
) -> Result<(SplatImage, SplatRenderStats), &'static str> {
    if width > i32::MAX as u32 || height > i32::MAX as u32 {
        return Err("Buffer dimensions too large (max i32::MAX)");
    }
    let size = u64::from(width)
        .checked_mul(u64::from(height))
        .filter(|&s| u32::try_from(s).is_ok())
        .ok_or("Buffer size overflow")? as usize;

    let mut stats = SplatRenderStats {
        splats_input: splats.len(),
        ..SplatRenderStats::default()
    };

    let mut projected: Vec<ProjectedSplat> = Vec::with_capacity(splats.len());
    for (i, g) in splats.iter().enumerate() {
        match project_gaussian(g, camera, params.max_radius) {
            Some(mut p) => {
                p.source_index = i;
                projected.push(p);
            }
            None => stats.splats_culled += 1,
        }
    }
    stats.splats_projected = projected.len();

    // Stable sort: nearest first; equal depths keep input order.
    projected.sort_by(|a, b| {
        a.depth
            .partial_cmp(&b.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Premultiplied accumulation + per-pixel transmittance.
    let mut acc = vec![Color::TRANSPARENT; size];
    let mut transmittance = vec![1.0_f32; size];
    let cutoff = params.transmittance_cutoff.clamp(0.0, 1.0);
    let (w, h) = (width as i32, height as i32);

    for splat in &projected {
        // Pixel centres are at half-integers: pixel p is evaluated iff
        // |p + 0.5 - centre| <= radius on each axis.
        let r = splat.radius as f32;
        let x0 = (splat.center.x - r - 0.5).ceil() as i32;
        let x1 = (splat.center.x + r - 0.5).floor() as i32;
        let y0 = (splat.center.y - r - 0.5).ceil() as i32;
        let y1 = (splat.center.y + r - 0.5).floor() as i32;
        let (x0, x1) = (x0.max(0), x1.min(w - 1));
        let (y0, y1) = (y0.max(0), y1.min(h - 1));
        if x0 > x1 || y0 > y1 {
            continue;
        }
        for py in y0..=y1 {
            let dy = py as f32 + 0.5 - splat.center.y;
            for px in x0..=x1 {
                let idx = (py * w + px) as usize;
                let t = transmittance[idx];
                if t <= cutoff {
                    stats.fragments_skipped_saturated += 1;
                    continue;
                }
                let dx = px as f32 + 0.5 - splat.center.x;
                let alpha = splat.alpha_at_offset(dx, dy);
                stats.fragments_evaluated += 1;
                if alpha < ALPHA_CUTOFF {
                    continue;
                }
                let a = acc[idx];
                acc[idx] = Color::new(
                    a.r + t * alpha * splat.color.r,
                    a.g + t * alpha * splat.color.g,
                    a.b + t * alpha * splat.color.b,
                    a.a + t * alpha,
                );
                transmittance[idx] = t * (1.0 - alpha);
                stats.fragments_blended += 1;
            }
        }
    }

    // Composite over the background and un-premultiply.
    let bg = params.background.premultiply();
    let mut pixels = Vec::with_capacity(size);
    for (i, a) in acc.iter().enumerate() {
        let t = transmittance[i];
        if t < 1.0 {
            stats.pixels_painted += 1;
        }
        let out_a = a.a + t * bg.a;
        let out = if out_a > 1e-6 {
            Color::new(
                (a.r + t * bg.r) / out_a,
                (a.g + t * bg.g) / out_a,
                (a.b + t * bg.b) / out_a,
                out_a,
            )
        } else {
            Color::TRANSPARENT
        };
        pixels.push(out);
    }

    Ok((
        SplatImage {
            width,
            height,
            pixels,
        },
        stats,
    ))
}

/// Render straight into a framebuffer sized by the framebuffer itself.
/// Convenience wrapper over [`render_gaussians`] +
/// [`SplatImage::write_to_framebuffer`].
///
/// # Panics
///
/// Panics only if the framebuffer's own dimensions fail the
/// [`render_gaussians`] size checks, which a constructed
/// [`Framebuffer`] has already passed.
pub fn render_gaussians_to_framebuffer(
    splats: &[Gaussian],
    camera: &SplatCamera,
    params: &SplatRenderParams,
    fb: &mut Framebuffer,
) -> SplatRenderStats {
    let (img, stats) = render_gaussians(splats, camera, params, fb.width(), fb.height())
        .expect("framebuffer dimensions are valid by construction");
    img.write_to_framebuffer(fb);
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    /// Identity view (world == view), 100×100, 90° fov: focal 50 px,
    /// principal point (50, 50). Depths read straight off `-z`.
    fn cam() -> SplatCamera {
        SplatCamera::from_fov_y(Mat4::identity(), 100, 100, FRAC_PI_2, 0.1, 100.0)
    }

    fn approx(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn centered_splat_projects_to_the_principal_point() {
        let g = Gaussian::isotropic(Vec3::new(0.0, 0.0, -6.0), 0.6, Color::RED, 1.0);
        let p = project_gaussian(&g, &cam(), DEFAULT_MAX_RADIUS).unwrap();
        assert!(approx(p.center.x, 50.0, 1e-4), "cx={}", p.center.x);
        assert!(approx(p.center.y, 50.0, 1e-4), "cy={}", p.center.y);
        assert!(approx(p.depth, 6.0, 1e-6));
        // Screen sigma = focal * scale / depth = 50 * 0.6 / 6 = 5 px.
        assert!(
            approx(p.cov[0], 25.0 + LOW_PASS_FILTER, 1e-3),
            "cov={:?}",
            p.cov
        );
        assert!(approx(p.cov[2], 25.0 + LOW_PASS_FILTER, 1e-3));
        assert!(approx(p.cov[1], 0.0, 1e-4));
        // 3-sigma radius of sigma ~5.03 is 16 px (ceil of 15.09).
        assert_eq!(p.radius, 16);
    }

    #[test]
    fn off_axis_points_project_with_y_down() {
        // Point level with the right edge: x == depth → u = cx + focal.
        let g = Gaussian::isotropic(Vec3::new(6.0, 0.0, -6.0), 0.1, Color::RED, 1.0);
        let p = project_gaussian(&g, &cam(), DEFAULT_MAX_RADIUS).unwrap();
        assert!(approx(p.center.x, 100.0, 1e-3), "cx={}", p.center.x);
        // Point level with the top edge: y == depth → v = cy - focal = 0.
        let g = Gaussian::isotropic(Vec3::new(0.0, 6.0, -6.0), 0.1, Color::RED, 1.0);
        let p = project_gaussian(&g, &cam(), DEFAULT_MAX_RADIUS).unwrap();
        assert!(approx(p.center.y, 0.0, 1e-3), "cy={}", p.center.y);
    }

    #[test]
    fn splats_behind_near_beyond_far_and_non_finite_cull() {
        let c = cam();
        let behind = Gaussian::isotropic(Vec3::new(0.0, 0.0, 6.0), 1.0, Color::RED, 1.0);
        assert!(project_gaussian(&behind, &c, 64).is_none());
        let too_close = Gaussian::isotropic(Vec3::new(0.0, 0.0, -0.05), 1.0, Color::RED, 1.0);
        assert!(project_gaussian(&too_close, &c, 64).is_none());
        let too_far = Gaussian::isotropic(Vec3::new(0.0, 0.0, -200.0), 1.0, Color::RED, 1.0);
        assert!(project_gaussian(&too_far, &c, 64).is_none());
        let nan = Gaussian::isotropic(Vec3::new(f32::NAN, 0.0, -6.0), 1.0, Color::RED, 1.0);
        assert!(project_gaussian(&nan, &c, 64).is_none());
        let mut bad_opacity = Gaussian::isotropic(Vec3::new(0.0, 0.0, -6.0), 1.0, Color::RED, 1.0);
        bad_opacity.opacity = f32::NAN; // field write bypasses the constructor clamp
        assert!(project_gaussian(&bad_opacity, &c, 64).is_none());
    }

    #[test]
    fn alpha_is_opacity_at_center_and_gaussian_off_it() {
        let g = Gaussian::isotropic(Vec3::new(0.0, 0.0, -6.0), 0.6, Color::RED, 0.8);
        let p = project_gaussian(&g, &cam(), DEFAULT_MAX_RADIUS).unwrap();
        assert!(approx(p.alpha_at_offset(0.0, 0.0), 0.8, 1e-6));
        // True screen sigma is 5 px; the effective falloff uses
        // sigma^2 + low-pass, so at one true sigma:
        let sigma = 5.0;
        let expected = 0.8 * (-0.5 * sigma * sigma / (sigma * sigma + LOW_PASS_FILTER)).exp();
        assert!(
            approx(p.alpha_at_offset(sigma, 0.0), expected, 1e-4),
            "alpha={} expected={expected}",
            p.alpha_at_offset(sigma, 0.0)
        );
        // Symmetric and strictly decreasing along a ray.
        assert!(approx(
            p.alpha_at_offset(3.0, -2.0),
            p.alpha_at_offset(-3.0, 2.0),
            1e-6
        ));
        let mut prev = 1.0_f32;
        for d in [0.0, 1.0, 2.0, 4.0, 8.0] {
            let a = p.alpha_at_offset(d, 0.0);
            assert!(a < prev || d == 0.0, "not decreasing at {d}: {a} vs {prev}");
            assert!(a <= 0.8);
            prev = a;
        }
    }

    #[test]
    fn anisotropic_splat_stretches_along_its_long_axis() {
        // Scale (1.2, 0.3) at depth 6, focal 50: sigma_x = 10, sigma_y = 2.5.
        let g = Gaussian::new(
            Vec3::new(0.0, 0.0, -6.0),
            Quat::identity(),
            Vec3::new(1.2, 0.3, 0.3),
            Color::RED,
            1.0,
        );
        let p = project_gaussian(&g, &cam(), DEFAULT_MAX_RADIUS).unwrap();
        // Same normalised distance along each axis → same alpha.
        assert!(approx(
            p.alpha_at_offset(10.0, 0.0),
            p.alpha_at_offset(0.0, 2.5),
            0.02
        ));
        // The long axis reaches much farther in absolute pixels.
        assert!(p.alpha_at_offset(10.0, 0.0) > p.alpha_at_offset(0.0, 10.0) + 0.3);
        assert!(p.cov[0] > p.cov[2] * 10.0, "cov={:?}", p.cov);
    }

    #[test]
    fn rotating_the_splat_rotates_the_footprint() {
        let base = Gaussian::new(
            Vec3::new(0.0, 0.0, -6.0),
            Quat::identity(),
            Vec3::new(1.2, 0.3, 0.3),
            Color::RED,
            1.0,
        );
        let turned = Gaussian::new(
            Vec3::new(0.0, 0.0, -6.0),
            Quat::from_axis_angle(Vec3::Z, std::f32::consts::FRAC_PI_2),
            Vec3::new(1.2, 0.3, 0.3),
            Color::RED,
            1.0,
        );
        let (a, b) = (
            project_gaussian(&base, &cam(), DEFAULT_MAX_RADIUS).unwrap(),
            project_gaussian(&turned, &cam(), DEFAULT_MAX_RADIUS).unwrap(),
        );
        // 90° about the view axis swaps the screen axes.
        assert!(
            approx(a.cov[0], b.cov[2], 0.05),
            "{:?} vs {:?}",
            a.cov,
            b.cov
        );
        assert!(approx(a.cov[2], b.cov[0], 0.05));
    }

    #[test]
    fn farther_splat_of_same_world_size_is_smaller_on_screen() {
        let near = project_gaussian(
            &Gaussian::isotropic(Vec3::new(0.0, 0.0, -6.0), 0.6, Color::RED, 1.0),
            &cam(),
            DEFAULT_MAX_RADIUS,
        )
        .unwrap();
        let far = project_gaussian(
            &Gaussian::isotropic(Vec3::new(0.0, 0.0, -12.0), 0.6, Color::RED, 1.0),
            &cam(),
            DEFAULT_MAX_RADIUS,
        )
        .unwrap();
        // Doubling the depth quarters the variance.
        assert!(approx(
            near.cov[0] / (far.cov[0] - LOW_PASS_FILTER),
            4.0,
            0.1
        ));
        assert!(far.radius < near.radius);
    }

    #[test]
    fn radius_clamps_to_max_radius() {
        let huge = Gaussian::isotropic(Vec3::new(0.0, 0.0, -6.0), 60.0, Color::RED, 1.0);
        let p = project_gaussian(&huge, &cam(), 7).unwrap();
        assert_eq!(p.radius, 7);
    }

    #[test]
    fn zero_scale_splat_still_paints_via_the_low_pass_filter() {
        let g = Gaussian::isotropic(Vec3::new(0.0, 0.0, -6.0), 0.0, Color::RED, 1.0);
        let p = project_gaussian(&g, &cam(), DEFAULT_MAX_RADIUS).unwrap();
        assert!(approx(p.cov[0], LOW_PASS_FILTER, 1e-6));
        let (img, stats) =
            render_gaussians(&[g], &cam(), &SplatRenderParams::default(), 100, 100).unwrap();
        assert!(stats.pixels_painted > 0);
        // Centre-pixel alpha is exp(-0.5 * 0.5 / 0.3) ~= 0.43.
        assert!(img.pixel(49, 49).unwrap().r > 0.3);
    }

    #[test]
    fn empty_scene_is_pure_background() {
        let params = SplatRenderParams {
            background: Color::rgb(0.1, 0.2, 0.3),
            ..SplatRenderParams::default()
        };
        let (img, stats) = render_gaussians(&[], &cam(), &params, 8, 8).unwrap();
        assert_eq!(stats.splats_input, 0);
        assert_eq!(stats.pixels_painted, 0);
        for px in img.as_slice() {
            assert!(approx(px.r, 0.1, 1e-6) && approx(px.g, 0.2, 1e-6));
            assert!(approx(px.a, 1.0, 1e-6));
        }
    }

    #[test]
    fn opaque_near_splat_hides_everything_behind_it() {
        // A very wide near splat: at the centre pixel its alpha is
        // 1 - ~1e-5, so the transmittance left for everything behind it
        // is under the 1/255 cutoff and the far splat is never blended.
        let near = Gaussian::isotropic(Vec3::new(0.0, 0.0, -4.0), 12.0, Color::RED, 1.0);
        let far = Gaussian::isotropic(Vec3::new(0.0, 0.0, -8.0), 2.0, Color::BLUE, 1.0);
        let (img, stats) = render_gaussians(
            &[far, near], // submitted back-to-front on purpose: sort must fix it
            &cam(),
            &SplatRenderParams::default(),
            100,
            100,
        )
        .unwrap();
        let centre = img.pixel(50, 50).unwrap();
        // Any blue contribution at all breaks this: the far splat must
        // have been skipped by the transmittance early exit.
        assert!(approx(centre.r, 1.0, 1e-4), "centre={centre:?}");
        assert!(approx(centre.b, 0.0, 1e-5));
        assert!(approx(centre.a, 1.0, 1e-6)); // opaque bg fills the rest
        assert!(stats.fragments_skipped_saturated > 0);
    }

    #[test]
    fn nearer_colour_wins_regardless_of_depth_sign_conventions() {
        // Same geometry as above with colours swapped: the winner must
        // follow depth, not colour or input position.
        let near = Gaussian::isotropic(Vec3::new(0.0, 0.0, -4.0), 0.5, Color::BLUE, 1.0);
        let far = Gaussian::isotropic(Vec3::new(0.0, 0.0, -8.0), 2.0, Color::RED, 1.0);
        let (img, _) = render_gaussians(
            &[near, far],
            &cam(),
            &SplatRenderParams::default(),
            100,
            100,
        )
        .unwrap();
        let centre = img.pixel(50, 50).unwrap();
        assert!(centre.b > 0.99, "centre={centre:?}");
        assert!(centre.r < 0.01);
    }

    #[test]
    fn translucent_splats_stack_by_the_over_operator() {
        let one = Gaussian::isotropic(Vec3::new(0.0, 0.0, -4.0), 0.5, Color::WHITE, 0.5);
        let splats = [one, one, one];
        // Transparent background, so the output alpha is the splats'
        // own accumulated alpha with nothing filled in behind.
        let params = SplatRenderParams {
            background: Color::TRANSPARENT,
            ..SplatRenderParams::default()
        };
        let (img, _) = render_gaussians(&splats, &cam(), &params, 100, 100).unwrap();
        // All three share a centre, so at pixel (50,50)'s centre the
        // single-splat alpha `a` applies three times: out = 1-(1-a)^3
        // of white over transparency.
        let p = project_gaussian(&one, &cam(), DEFAULT_MAX_RADIUS).unwrap();
        let a = f64::from(p.alpha_at_offset(0.5, 0.5));
        let expected = 1.0 - (1.0 - a).powi(3);
        let centre = img.pixel(50, 50).unwrap();
        assert!(
            approx(centre.a, expected as f32, 1e-4),
            "a={} expected={expected}",
            centre.a
        );
        // White over black, un-premultiplied: the hue stays pure white.
        assert!(approx(centre.r, 1.0, 1e-4));
    }

    #[test]
    fn distinct_depths_render_identically_in_any_input_order() {
        let mk = |z: f32, s: f32, c: Color| Gaussian::isotropic(Vec3::new(0.3, -0.2, z), s, c, 0.7);
        let scene = [
            mk(-3.0, 0.4, Color::RED),
            mk(-5.0, 0.9, Color::GREEN),
            mk(-8.0, 1.6, Color::BLUE),
            mk(-4.0, 0.2, Color::YELLOW),
        ];
        let mut shuffled = scene;
        shuffled.reverse();
        let params = SplatRenderParams::default();
        let (a, _) = render_gaussians(&scene, &cam(), &params, 60, 60).unwrap();
        let (b, _) = render_gaussians(&shuffled, &cam(), &params, 60, 60).unwrap();
        assert_eq!(a.as_slice(), b.as_slice());
    }

    #[test]
    fn depth_ties_keep_input_order_deterministically() {
        let red = Gaussian::isotropic(Vec3::new(0.0, 0.0, -4.0), 0.5, Color::RED, 0.6);
        let blue = Gaussian::isotropic(Vec3::new(0.0, 0.0, -4.0), 0.5, Color::BLUE, 0.6);
        let params = SplatRenderParams::default();
        let (a, _) = render_gaussians(&[red, blue], &cam(), &params, 100, 100).unwrap();
        let (b, _) = render_gaussians(&[blue, red], &cam(), &params, 100, 100).unwrap();
        let (ca, cb) = (a.pixel(50, 50).unwrap(), b.pixel(50, 50).unwrap());
        assert!(ca.r > ca.b, "first-submitted red should lead: {ca:?}");
        assert!(cb.b > cb.r, "first-submitted blue should lead: {cb:?}");
    }

    #[test]
    fn fully_off_screen_splat_evaluates_no_fragments() {
        let off = Gaussian::isotropic(Vec3::new(60.0, 0.0, -6.0), 0.5, Color::RED, 1.0);
        let (_, stats) =
            render_gaussians(&[off], &cam(), &SplatRenderParams::default(), 100, 100).unwrap();
        assert_eq!(stats.splats_projected, 1);
        assert_eq!(stats.fragments_evaluated, 0);
        assert_eq!(stats.pixels_painted, 0);
    }

    #[test]
    fn edge_crossing_splat_still_paints_the_edge() {
        // Centre two pixels past the right edge, sigma 5 px: the
        // footprint reaches back onto the screen.
        let edge = Gaussian::isotropic(Vec3::new(6.12, 0.0, -6.0), 0.6, Color::RED, 1.0);
        let (img, stats) =
            render_gaussians(&[edge], &cam(), &SplatRenderParams::default(), 100, 100).unwrap();
        assert!(stats.pixels_painted > 0);
        assert!(img.pixel(99, 50).unwrap().r > 0.1);
        assert!(img.pixel(0, 50).unwrap().r < 0.01);
    }

    #[test]
    fn stats_count_projected_and_culled() {
        let good = Gaussian::isotropic(Vec3::new(0.0, 0.0, -6.0), 0.5, Color::RED, 1.0);
        let bad = Gaussian::isotropic(Vec3::new(0.0, 0.0, 6.0), 0.5, Color::RED, 1.0);
        let (_, stats) = render_gaussians(
            &[good, bad],
            &cam(),
            &SplatRenderParams::default(),
            100,
            100,
        )
        .unwrap();
        assert_eq!(stats.splats_input, 2);
        assert_eq!(stats.splats_projected, 1);
        assert_eq!(stats.splats_culled, 1);
        assert!(stats.fragments_blended > 0);
        assert!(stats.fragments_evaluated >= stats.fragments_blended);
    }

    #[test]
    fn view_matrix_moves_the_splat() {
        // Camera at z=+6 looking at the origin: a splat at the world
        // origin centres on screen and sits at depth 6.
        let view = Mat4::look_at(Vec3::new(0.0, 0.0, 6.0), Vec3::ZERO, Vec3::Y);
        let c = SplatCamera::from_fov_y(view, 100, 100, FRAC_PI_2, 0.1, 100.0);
        let g = Gaussian::isotropic(Vec3::ZERO, 0.6, Color::RED, 1.0);
        let p = project_gaussian(&g, &c, DEFAULT_MAX_RADIUS).unwrap();
        assert!(approx(p.center.x, 50.0, 1e-3));
        assert!(approx(p.center.y, 50.0, 1e-3));
        assert!(approx(p.depth, 6.0, 1e-4));
    }

    #[test]
    fn framebuffer_write_matches_the_image() {
        let splats = [Gaussian::isotropic(
            Vec3::new(0.0, 0.0, -4.0),
            0.5,
            Color::RED,
            1.0,
        )];
        let (img, _) =
            render_gaussians(&splats, &cam(), &SplatRenderParams::default(), 100, 100).unwrap();
        let mut fb = Framebuffer::new(100, 100).unwrap();
        img.write_to_framebuffer(&mut fb);
        let centre = img.pixel(50, 50).unwrap();
        assert_eq!(fb.get_pixel(50, 50), Some(centre.to_argb_u32()));
        let mut fb2 = Framebuffer::new(100, 100).unwrap();
        let stats = render_gaussians_to_framebuffer(
            &splats,
            &cam(),
            &SplatRenderParams::default(),
            &mut fb2,
        );
        assert_eq!(stats.splats_projected, 1);
        assert_eq!(fb2.get_pixel(50, 50), fb.get_pixel(50, 50));
    }
}
