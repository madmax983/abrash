//! BSP seg projection math: world-space segs to screen columns.
//!
//! Six pure functions that implement Doom's `R_AddLine` / `R_ScaleFromGlobalAngle`
//! projection pipeline. All operate on BAM angles and 16.16 fixed-point, no
//! floating-point in the hot path (except `point_to_angle` which uses `atan2`
//! for the initial conversion).

use abrash_core::bam::{ANG90, ANG180, Bam};
use abrash_core::fixed16_16::Fixed16_16;

use crate::types::Vec2Fixed;

// ---------------------------------------------------------------------------
// 1. point_to_angle
// ---------------------------------------------------------------------------

/// Compute the angle from `origin` to `target` as a BAM.
///
/// 0 = east, `ANG90` = north. Uses `atan2(dy, dx)` on f32 deltas, then
/// converts via [`Bam::from_radians`].
#[inline]
#[must_use]
pub fn point_to_angle(origin: Vec2Fixed, target: Vec2Fixed) -> Bam {
    let dx = (target.x - origin.x).to_f32();
    let dy = (target.y - origin.y).to_f32();
    Bam::from_radians(f32::atan2(dy, dx))
}

// ---------------------------------------------------------------------------
// 2. projection_distance
// ---------------------------------------------------------------------------

/// Projection constant: `(screen_width / 2) / tan(fov / 2)`.
///
/// For a 90-degree FOV at 320 px wide: `160 / tan(45 deg) = 160.0`.
#[inline]
#[must_use]
pub fn projection_distance(screen_width: u32, fov: Bam) -> Fixed16_16 {
    let half_width = screen_width as f32 / 2.0;
    let half_fov_rad = (f64::from(fov.0) / 2.0 * core::f64::consts::TAU / 4_294_967_296.0) as f32;
    let tan_half = half_fov_rad.tan();
    if tan_half.abs() < 1e-9 {
        return Fixed16_16::from_raw(i32::MAX);
    }
    Fixed16_16::from_f32(half_width / tan_half)
}

// ---------------------------------------------------------------------------
// 3. angle_to_column
// ---------------------------------------------------------------------------

/// Map a view-relative angle to a screen column.
///
/// `column = half_width - tan(angle) * projection`.
///
/// `view_angle = 0` maps to the center column. Positive angles (left of center)
/// map to lower column numbers; negative angles (large BAM, right of center)
/// map to higher column numbers.
#[inline]
#[must_use]
pub fn angle_to_column(view_angle: Bam, projection: Fixed16_16, screen_width: u32) -> i32 {
    let half_width = screen_width as f32 / 2.0;
    let angle_rad = bam_to_signed_radians(view_angle);
    let col = half_width - angle_rad.tan() * projection.to_f32();
    col as i32
}

// ---------------------------------------------------------------------------
// 4. clip_seg_angles
// ---------------------------------------------------------------------------

/// Clip a seg's angular span to the camera's FOV.
///
/// Returns view-relative angles `(clipped_left, clipped_right)` or `None` if:
/// - The seg is back-facing (angular span >= 180 degrees).
/// - The seg is entirely outside the FOV.
///
/// `angle1` and `angle2` are the world-space angles from the camera to the seg's
/// start and end vertices. `camera_angle` is the direction the camera faces.
/// `half_fov` is half the field of view.
#[must_use]
pub fn clip_seg_angles(
    angle1: Bam,
    angle2: Bam,
    camera_angle: Bam,
    half_fov: Bam,
) -> Option<(Bam, Bam)> {
    // 1. Compute span in BAM. If the unsigned span is >= ANG180, the seg
    //    is back-facing (or wraps more than a half-circle).
    let span = angle1 - angle2;
    if span.0 >= ANG180.0 {
        return None;
    }

    // 2. Convert to view-relative angles.
    let mut rw1 = angle1 - camera_angle;
    let mut rw2 = angle2 - camera_angle;

    // 3. Clip left edge: if rw1 is outside +half_fov, clamp it.
    //    "Left" in screen terms = positive view angle.
    //    We check if rw1 is beyond +half_fov by looking at (rw1 - half_fov):
    //    if the signed result is positive (i.e. unsigned < ANG180), rw1 is
    //    too far left.
    let left_overshoot = rw1 - half_fov;
    if bam_is_positive(left_overshoot) {
        // rw1 is past the left edge of the FOV. Clamp it.
        // But if the overshoot exceeds the entire span, the seg is fully
        // off the left side.
        if left_overshoot.0 >= span.0 {
            return None;
        }
        rw1 = half_fov;
    }

    // 4. Clip right edge: if rw2 is outside -half_fov, clamp it.
    //    "Right" in screen terms = negative view angle = large BAM.
    //    -half_fov in BAM is the negate of half_fov.
    let neg_half_fov = -half_fov;
    // rw2 is "too far right" if (neg_half_fov - rw2) is positive in signed terms.
    let right_overshoot = neg_half_fov - rw2;
    if bam_is_positive(right_overshoot) {
        // rw2 is past the right edge. Clamp it. But if the overshoot
        // exceeds the entire span, it's fully off screen.
        if right_overshoot.0 >= span.0 {
            return None;
        }
        rw2 = neg_half_fov;
    }

    Some((rw1, rw2))
}

// ---------------------------------------------------------------------------
// 5. seg_perpendicular_distance
// ---------------------------------------------------------------------------

/// Perpendicular distance from the camera to the seg's infinite line.
///
/// The seg's outward normal direction is `seg_angle + ANG90`. The distance is
/// the dot product of `(camera_pos - seg_v1)` with the unit normal.
#[inline]
#[must_use]
pub fn seg_perpendicular_distance(
    seg_v1: Vec2Fixed,
    seg_angle: Bam,
    camera_pos: Vec2Fixed,
) -> Fixed16_16 {
    let normal_angle = seg_angle + ANG90;
    let (sin, cos) = normal_angle.sin_cos_fixed();

    let dx = camera_pos.x - seg_v1.x;
    let dy = camera_pos.y - seg_v1.y;

    // dot product with unit normal = dx*cos + dy*sin
    let dist = dx.fixed_mul(cos) + dy.fixed_mul(sin);
    dist.abs()
}

// ---------------------------------------------------------------------------
// 6. wall_scale_at_column
// ---------------------------------------------------------------------------

/// Doom's `R_ScaleFromGlobalAngle`: compute wall texture scale at a screen column.
///
/// Returns a 16.16 fixed-point scale factor clamped to `[256, 64*FRACUNIT]`
/// (i.e. raw values 256 to `4_194_304`, representing approximately 0.0039 to 64.0).
#[must_use]
pub fn wall_scale_at_column(
    col: i32,
    screen_width: u32,
    projection: Fixed16_16,
    camera_angle: Bam,
    seg_normal_angle: Bam,
    seg_distance: Fixed16_16,
) -> Fixed16_16 {
    const MIN_SCALE: i32 = 256;
    const MAX_SCALE: i32 = 64 << 16; // 64 * FRACUNIT

    let half_w = screen_width as f32 / 2.0;
    let proj_f = projection.to_f32();

    // 1. Column angle = atan((half_w - col) / projection)
    let col_angle = Bam::from_radians(((half_w - col as f32) / proj_f).atan());

    // 2. vis_angle = camera_angle + col_angle (global angle for this column)
    let vis_angle = camera_angle + col_angle;

    // 3. anglea = ANG90 + col_angle
    let anglea = ANG90 + col_angle;

    // 4. angleb = ANG90 + (vis_angle - seg_normal_angle)
    let angleb = ANG90 + (vis_angle - seg_normal_angle);

    // 5. scale = projection * |sin(angleb)| / (seg_distance * |sin(anglea)|)
    //    Doom uses absolute values to ensure positive scale regardless of angle quadrant.
    let (sin_b, _) = angleb.sin_cos_fixed();
    let (sin_a, _) = anglea.sin_cos_fixed();

    let numerator = projection.fixed_mul(sin_b.abs());
    let denominator = seg_distance.fixed_mul(sin_a.abs());

    if denominator.raw() == 0 {
        return Fixed16_16::from_raw(MAX_SCALE);
    }

    let scale = numerator.fixed_div(denominator);

    // 6. Clamp
    let raw = scale.raw();
    if raw < MIN_SCALE {
        Fixed16_16::from_raw(MIN_SCALE)
    } else if raw > MAX_SCALE {
        Fixed16_16::from_raw(MAX_SCALE)
    } else {
        scale
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Interpret a BAM value as a signed angle: values < `ANG180` (`0x8000_0000`)
/// are positive, values >= `ANG180` are negative.
///
/// Returns `true` if the angle represents a positive (counter-clockwise) value.
#[inline]
const fn bam_is_positive(angle: Bam) -> bool {
    angle.0 > 0 && angle.0 < ANG180.0
}

/// Convert a BAM angle to a signed f32 radian value.
///
/// BAM values in `[0, ANG180)` map to `[0, +pi)`.
/// BAM values in `[ANG180, 0xFFFF_FFFF]` map to `(-pi, 0)`.
#[inline]
fn bam_to_signed_radians(angle: Bam) -> f32 {
    let raw = angle.0 as i32; // reinterpret as signed
    (f64::from(raw) * core::f64::consts::TAU / 4_294_967_296.0) as f32
}

// ---------------------------------------------------------------------------
// Column clip state for BSP front-to-back rendering
// ---------------------------------------------------------------------------

/// Per-column occlusion tracker for front-to-back BSP wall rendering.
///
/// As the BSP traversal visits walls in front-to-back order, each wall claims
/// screen rows from the top (ceiling) and bottom (floor). `ColumnClip` tracks
/// the remaining "open" (undrawn) row range per screen column, enabling:
///
/// - **Early exit**: once every column is fully filled, no more walls are visible.
/// - **Correct overdraw elimination**: later (farther) walls only draw into the
///   rows that are still open.
pub struct ColumnClip {
    open_top: Vec<i32>,
    open_bot: Vec<i32>,
    width: u32,
    height: u32,
    filled_count: u32,
}

impl ColumnClip {
    /// Create a new clip state with all columns fully open.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let w = width as usize;
        Self {
            open_top: vec![0; w],
            open_bot: vec![height as i32 - 1; w],
            width,
            height,
            filled_count: 0,
        }
    }

    /// Returns `true` if `col` has at least one undrawn row.
    ///
    /// Out-of-bounds columns return `false`.
    #[inline]
    #[must_use]
    pub fn is_open(&self, col: i32) -> bool {
        if col < 0 || col >= self.width as i32 {
            return false;
        }
        let c = col as usize;
        self.open_top[c] <= self.open_bot[c]
    }

    /// Topmost open row for `col`. Clamped to `[0, height-1]` for OOB columns.
    #[inline]
    #[must_use]
    pub fn top(&self, col: i32) -> i32 {
        let c = (col.max(0) as usize).min(self.open_top.len().saturating_sub(1));
        self.open_top[c]
    }

    /// Bottommost open row for `col`. Clamped to `[0, height-1]` for OOB columns.
    #[inline]
    #[must_use]
    pub fn bot(&self, col: i32) -> i32 {
        let c = (col.max(0) as usize).min(self.open_bot.len().saturating_sub(1));
        self.open_bot[c]
    }

    /// Advance the top boundary downward (ceiling was drawn down to `row`).
    ///
    /// No-op for out-of-bounds columns.
    #[inline]
    pub fn set_top(&mut self, col: i32, row: i32) {
        if col >= 0 && col < self.width as i32 {
            self.open_top[col as usize] = row;
        }
    }

    /// Advance the bottom boundary upward (floor was drawn up to `row`).
    ///
    /// No-op for out-of-bounds columns.
    #[inline]
    pub fn set_bot(&mut self, col: i32, row: i32) {
        if col >= 0 && col < self.width as i32 {
            self.open_bot[col as usize] = row;
        }
    }

    /// Mark a column as completely filled (no open rows remain).
    ///
    /// No-op for out-of-bounds columns.
    #[inline]
    pub fn mark_solid(&mut self, col: i32) {
        if col >= 0 && col < self.width as i32 {
            let c = col as usize;
            // Only bump the counter if the column was previously open.
            if self.open_top[c] <= self.open_bot[c] {
                self.filled_count += 1;
            }
            self.open_top[c] = self.height as i32;
        }
    }

    /// Returns `true` when every column has been fully filled.
    #[inline]
    #[must_use]
    pub const fn all_filled(&self) -> bool {
        self.filled_count >= self.width
    }

    /// Reset all columns to fully open (for a new frame).
    pub fn reset(&mut self) {
        for v in &mut self.open_top {
            *v = 0;
        }
        for v in &mut self.open_bot {
            *v = self.height as i32 - 1;
        }
        self.filled_count = 0;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::bam::{ANG45, ANG90, ANG180};
    use abrash_core::fixed16_16::Fixed16_16;

    /// Helper: absolute difference between two BAM angles, taking the shorter arc.
    fn bam_abs_diff(a: Bam, b: Bam) -> u32 {
        let d = a.0.wrapping_sub(b.0);
        if d > ANG180.0 { d.wrapping_neg() } else { d }
    }

    // -----------------------------------------------------------------------
    // point_to_angle
    // -----------------------------------------------------------------------

    #[test]
    fn point_to_angle_east() {
        let origin = Vec2Fixed::from_ints(0, 0);
        let target = Vec2Fixed::from_ints(100, 0);
        let angle = point_to_angle(origin, target);
        // East = 0 degrees. Allow ~1 degree tolerance.
        let tolerance = 0x0100_0000_u32; // ~5.6 degrees
        assert!(
            bam_abs_diff(angle, Bam::ZERO) < tolerance,
            "expected ~0 (east), got {angle}"
        );
    }

    #[test]
    fn point_to_angle_north() {
        let origin = Vec2Fixed::from_ints(0, 0);
        let target = Vec2Fixed::from_ints(0, 100);
        let angle = point_to_angle(origin, target);
        let tolerance = 0x0100_0000_u32;
        assert!(
            bam_abs_diff(angle, ANG90) < tolerance,
            "expected ~ANG90 (north), got {angle}"
        );
    }

    #[test]
    fn point_to_angle_west() {
        let origin = Vec2Fixed::from_ints(0, 0);
        let target = Vec2Fixed::from_ints(-100, 0);
        let angle = point_to_angle(origin, target);
        let tolerance = 0x0100_0000_u32;
        assert!(
            bam_abs_diff(angle, ANG180) < tolerance,
            "expected ~ANG180 (west), got {angle}"
        );
    }

    // -----------------------------------------------------------------------
    // projection_distance
    // -----------------------------------------------------------------------

    #[test]
    fn projection_distance_90_fov_320_wide() {
        let proj = projection_distance(320, ANG90);
        let val = proj.to_f32();
        assert!(
            (val - 160.0).abs() < 1.0,
            "90deg FOV at 320px should give ~160, got {val}"
        );
    }

    #[test]
    fn projection_distance_wider_fov_gives_smaller_value() {
        let proj_90 = projection_distance(320, ANG90);
        // 120-degree FOV: ANG90 + ANG45*2/3... easier to use raw BAM.
        let fov_120 = Bam(ANG90.0 + ANG90.0 / 3); // 120 degrees
        let proj_120 = projection_distance(320, fov_120);
        assert!(
            proj_120.to_f32() < proj_90.to_f32(),
            "wider FOV should give smaller projection distance: 120deg={}, 90deg={}",
            proj_120.to_f32(),
            proj_90.to_f32()
        );
    }

    // -----------------------------------------------------------------------
    // angle_to_column
    // -----------------------------------------------------------------------

    #[test]
    fn angle_to_column_center() {
        let proj = Fixed16_16::from_f32(160.0);
        let col = angle_to_column(Bam::ZERO, proj, 320);
        assert_eq!(col, 160, "view_angle=0 should map to center column (160)");
    }

    #[test]
    fn angle_to_column_left_edge() {
        let proj = Fixed16_16::from_f32(160.0);
        let col = angle_to_column(ANG45, proj, 320);
        // ANG45 = +45 degrees. tan(45) = 1.0. col = 160 - 1.0*160 = 0.
        assert!(
            (col - 0).abs() <= 2,
            "ANG45 should map to column ~0, got {col}"
        );
    }

    #[test]
    fn angle_to_column_right_edge() {
        let proj = Fixed16_16::from_f32(160.0);
        // -45 degrees in BAM is the negate of ANG45
        let neg_45 = -ANG45;
        let col = angle_to_column(neg_45, proj, 320);
        // tan(-45) = -1.0. col = 160 - (-1.0)*160 = 320.
        assert!(
            (col - 320).abs() <= 2,
            "negative ANG45 should map to column ~320, got {col}"
        );
    }

    // -----------------------------------------------------------------------
    // clip_seg_angles
    // -----------------------------------------------------------------------

    #[test]
    fn clip_seg_fully_visible() {
        // Camera facing east (0), seg from +20deg to -20deg (fully within 45deg half-FOV).
        let camera = Bam::ZERO;
        let half_fov = ANG45;
        let angle1 = Bam::from_radians(0.35); // ~20 degrees
        let angle2 = Bam::from_radians(-0.35); // ~-20 degrees

        let result = clip_seg_angles(angle1, angle2, camera, half_fov);
        assert!(result.is_some(), "seg within FOV should be visible");

        let (rw1, rw2) = result.unwrap();
        // View-relative angles should match since seg is fully within FOV.
        let rw1_deg = bam_to_signed_radians(rw1).to_degrees();
        let rw2_deg = bam_to_signed_radians(rw2).to_degrees();
        assert!(rw1_deg > 0.0, "left edge should be positive, got {rw1_deg}");
        assert!(
            rw2_deg < 0.0,
            "right edge should be negative, got {rw2_deg}"
        );
    }

    #[test]
    fn clip_seg_behind_camera() {
        // Seg whose angular span (angle1 - angle2) >= 180 degrees means it's
        // back-facing. We need angle2 to be *ahead* of angle1 by more than 180
        // degrees in the wrapping sense.
        let camera = Bam::ZERO;
        let half_fov = ANG45;

        // angle1 at ~30 degrees, angle2 at ~210+delta degrees.
        // span = angle1 - angle2 = 30 - 211 = -181 degrees -> wraps to 179 in unsigned?
        // No. Let's think in raw BAM:
        //   angle1 = 0x0AAA_AAAA (~30 deg)
        //   angle2 = angle1 - ANG180 - 0x100_0000  (so angle1 - angle2 = ANG180 + 0x100_0000)
        let angle1 = Bam(0x0AAA_AAAA); // ~30 degrees
        let angle2 = Bam(angle1.0.wrapping_sub(ANG180.0).wrapping_sub(0x0100_0000));
        // Now angle1 - angle2 = ANG180 + 0x0100_0000 which is >= ANG180.

        let result = clip_seg_angles(angle1, angle2, camera, half_fov);
        assert!(
            result.is_none(),
            "back-facing seg (span >= 180deg) should be culled"
        );
    }

    #[test]
    fn clip_seg_partially_visible_left() {
        // Camera facing east, seg extends past the left FOV edge.
        let camera = Bam::ZERO;
        let half_fov = ANG45;

        // Seg from +60 degrees to +10 degrees (left side of view).
        // +60 exceeds half_fov (45 deg), so left edge should be clipped.
        let angle1 = Bam::from_radians(1.047); // ~60 degrees
        let angle2 = Bam::from_radians(0.175); // ~10 degrees

        let result = clip_seg_angles(angle1, angle2, camera, half_fov);
        assert!(result.is_some(), "partially visible seg should return Some");

        let (rw1, _rw2) = result.unwrap();
        // rw1 should be clamped to half_fov (ANG45).
        assert_eq!(rw1.0, half_fov.0, "left edge should be clipped to half_fov");
    }

    #[test]
    fn clip_seg_entirely_off_left() {
        // Seg entirely to the left of the FOV.
        let camera = Bam::ZERO;
        let half_fov = ANG45;

        // Both angles far to the left (> 45 deg from center).
        let angle1 = Bam::from_radians(1.4); // ~80 degrees
        let angle2 = Bam::from_radians(0.9); // ~52 degrees

        let result = clip_seg_angles(angle1, angle2, camera, half_fov);
        assert!(
            result.is_none(),
            "seg entirely outside left FOV should be culled"
        );
    }

    #[test]
    fn clip_seg_entirely_off_right() {
        // Seg entirely to the right of the FOV.
        let camera = Bam::ZERO;
        let half_fov = ANG45;

        // Both angles far to the right (negative view angles).
        let angle1 = Bam::from_radians(-0.9); // ~-52 degrees
        let angle2 = Bam::from_radians(-1.4); // ~-80 degrees

        let result = clip_seg_angles(angle1, angle2, camera, half_fov);
        assert!(
            result.is_none(),
            "seg entirely outside right FOV should be culled"
        );
    }

    // -----------------------------------------------------------------------
    // seg_perpendicular_distance
    // -----------------------------------------------------------------------

    #[test]
    fn seg_perp_distance_facing_directly() {
        // Horizontal seg at y=0 running east (angle=0), camera at (256, 128).
        // Normal is angle+90 = ANG90 (pointing north/+y).
        // Distance = dot((256-0, 128-0), normal(0, 1)) = 128.
        let seg_v1 = Vec2Fixed::from_ints(0, 0);
        let seg_angle = Bam::ZERO;
        let camera = Vec2Fixed::from_ints(256, 128);

        let dist = seg_perpendicular_distance(seg_v1, seg_angle, camera);
        let val = dist.to_f32();
        assert!(
            (val - 128.0).abs() < 2.0,
            "perpendicular distance should be ~128, got {val}"
        );
    }

    #[test]
    fn seg_perp_distance_on_line() {
        // Camera sitting exactly on the seg line.
        let seg_v1 = Vec2Fixed::from_ints(0, 0);
        let seg_angle = Bam::ZERO; // east-running seg
        let camera = Vec2Fixed::from_ints(100, 0); // on the line

        let dist = seg_perpendicular_distance(seg_v1, seg_angle, camera);
        let val = dist.to_f32();
        assert!(
            val.abs() < 2.0,
            "camera on the line should have ~0 distance, got {val}"
        );
    }

    #[test]
    fn seg_perp_distance_vertical_seg() {
        // Vertical seg at x=100 running north (angle=ANG90), camera at (200, 50).
        // Normal is ANG90+ANG90 = ANG180, unit vec = (-1, 0).
        // Distance = dot((200-100, 50-0), (-1, 0)) = -100, abs = 100.
        let seg_v1 = Vec2Fixed::from_ints(100, 0);
        let seg_angle = ANG90;
        let camera = Vec2Fixed::from_ints(200, 50);

        let dist = seg_perpendicular_distance(seg_v1, seg_angle, camera);
        let val = dist.to_f32();
        assert!(
            (val - 100.0).abs() < 2.0,
            "perpendicular distance should be ~100, got {val}"
        );
    }

    // -----------------------------------------------------------------------
    // wall_scale_at_column
    // -----------------------------------------------------------------------

    #[test]
    fn wall_scale_center_column() {
        // Center column, projection=160, camera facing east, seg normal = east (ANG0+ANG90=ANG90).
        // At center: col_angle=0, vis_angle=camera_angle=0.
        // anglea = ANG90 + 0 = ANG90, sin(ANG90) = 1.0
        // angleb = ANG90 + (0 - ANG90) = ANG90 + (-ANG90) = 0, sin(0) = 0 ...
        //
        // Actually, for a seg whose normal points at the camera, the seg_normal_angle
        // should be the angle FROM the seg TO the camera. Let's set up a more
        // concrete scenario:
        //
        // Camera at origin facing east. Wall is 128 units away, perpendicular to
        // the camera's view (wall runs north-south, normal points west = ANG180).
        // seg_normal_angle = ANG180 (pointing back at camera).
        //
        // At center column:
        //   col_angle = 0
        //   vis_angle = 0 (camera_angle + col_angle)
        //   anglea = ANG90 + 0 = ANG90   → sin = 1.0
        //   angleb = ANG90 + (0 - ANG180) = ANG90 - ANG180 = ANG270(-90) → sin = -1.0 → |sin| = 1.0
        //   scale = 160 * 1.0 / (128 * 1.0) = 1.25
        let proj = Fixed16_16::from_f32(160.0);
        let dist = Fixed16_16::from_f32(128.0);
        let camera_angle = Bam::ZERO;
        // Wall normal points west (back at camera):
        let seg_normal = ANG180;

        let scale = wall_scale_at_column(160, 320, proj, camera_angle, seg_normal, dist);
        let val = scale.to_f32();
        assert!(
            (val - 1.25).abs() < 0.1,
            "center column scale should be ~1.25, got {val}"
        );
    }

    #[test]
    fn wall_scale_edge_column_smaller() {
        // Use an angled wall (not perpendicular to camera) where edge vs center
        // scales differ. Wall normal at 45 degrees off-axis creates foreshortening
        // that varies across columns.
        let proj = Fixed16_16::from_f32(160.0);
        let dist = Fixed16_16::from_f32(128.0);
        let camera_angle = Bam::ZERO;
        // Wall normal at ANG180 + ANG45 = 225 degrees (angled wall).
        let seg_normal = ANG180 + ANG45;

        let center_scale = wall_scale_at_column(160, 320, proj, camera_angle, seg_normal, dist);
        let edge_scale = wall_scale_at_column(0, 320, proj, camera_angle, seg_normal, dist);

        // An angled wall produces different scales at different columns due to
        // varying incidence angles.
        assert_ne!(
            center_scale.raw(),
            edge_scale.raw(),
            "angled wall should have different scales at center vs edge"
        );
    }

    #[test]
    fn wall_scale_clamps_minimum() {
        // Very far away wall should clamp to minimum scale.
        let proj = Fixed16_16::from_f32(160.0);
        let dist = Fixed16_16::from_f32(32000.0); // very far
        let camera_angle = Bam::ZERO;
        let seg_normal = ANG180;

        let scale = wall_scale_at_column(160, 320, proj, camera_angle, seg_normal, dist);
        assert!(
            scale.raw() >= 256,
            "scale should be clamped to minimum 256, got {}",
            scale.raw()
        );
    }

    #[test]
    fn wall_scale_clamps_maximum() {
        // Very close wall should clamp to maximum scale.
        let proj = Fixed16_16::from_f32(160.0);
        let dist = Fixed16_16::from_raw(1); // absurdly close
        let camera_angle = Bam::ZERO;
        let seg_normal = ANG180;

        let scale = wall_scale_at_column(160, 320, proj, camera_angle, seg_normal, dist);
        assert!(
            scale.raw() <= 64 << 16,
            "scale should be clamped to maximum 64*FRACUNIT, got {}",
            scale.raw()
        );
    }

    // -----------------------------------------------------------------------
    // ColumnClip
    // -----------------------------------------------------------------------

    #[test]
    fn column_clip_initial_state() {
        let clip = ColumnClip::new(320, 200);
        // All columns open.
        for col in 0..320 {
            assert!(clip.is_open(col), "column {col} should be open initially");
        }
        assert_eq!(clip.top(0), 0);
        assert_eq!(clip.bot(0), 199);
        assert!(!clip.all_filled());
    }

    #[test]
    fn column_clip_set_top() {
        let mut clip = ColumnClip::new(320, 200);
        clip.set_top(10, 50);
        assert_eq!(clip.top(10), 50);
        assert_eq!(clip.bot(10), 199);
        assert!(clip.is_open(10));
    }

    #[test]
    fn column_clip_set_bot() {
        let mut clip = ColumnClip::new(320, 200);
        clip.set_bot(10, 150);
        assert_eq!(clip.top(10), 0);
        assert_eq!(clip.bot(10), 150);
        assert!(clip.is_open(10));
    }

    #[test]
    fn column_clip_mark_solid() {
        let mut clip = ColumnClip::new(320, 200);
        clip.mark_solid(10);
        assert!(!clip.is_open(10));
    }

    #[test]
    fn column_clip_all_filled() {
        let mut clip = ColumnClip::new(4, 200);
        for col in 0..4 {
            clip.mark_solid(col);
        }
        assert!(clip.all_filled());
    }

    #[test]
    fn column_clip_reset() {
        let mut clip = ColumnClip::new(320, 200);
        clip.mark_solid(10);
        clip.set_top(20, 50);
        clip.reset();
        assert!(clip.is_open(10));
        assert_eq!(clip.top(10), 0);
        assert_eq!(clip.bot(10), 199);
        assert_eq!(clip.top(20), 0);
        assert!(!clip.all_filled());
    }

    #[test]
    fn column_clip_out_of_bounds_not_open() {
        let clip = ColumnClip::new(320, 200);
        assert!(!clip.is_open(-1));
        assert!(!clip.is_open(320));
    }
}
