//! DDA (Digital Differential Analyzer) grid stepper.
//!
//! The core algorithm that steps through grid cells along a ray. This is the
//! engine shared by all three query tiers (`cast_los`, `cast_ray`,
//! `cast_ray_detailed`).
//!
//! The DDA works in fixed-point arithmetic using `Fixed16_16` for all distance
//! calculations and `Bam` for angles. One table lookup per ray via
//! `sin_cos_fixed()`.

use abrash_core::bam::Bam;
use abrash_core::fixed16_16::{FIXED_ONE, Fixed16_16};

use crate::types::{Side, Vec2Fixed};

/// A very large fixed-point value used as "infinity" when a ray component is zero.
/// We use `i32::MAX` to avoid overflow in comparisons while still being a valid
/// `Fixed16_16`.
const FIXED_INFINITY: Fixed16_16 = Fixed16_16::from_raw(i32::MAX);

/// DDA grid stepper: steps through grid cells along a ray direction.
///
/// Constructed from an origin position and a BAM angle. Each call to `step`
/// advances to the next grid cell boundary and reports which cell face was hit.
pub(crate) struct DdaStepper {
    /// Current cell X coordinate.
    pub cell_x: i32,
    /// Current cell Y coordinate.
    pub cell_y: i32,
    /// Step direction in X: +1 or -1.
    step_x: i32,
    /// Step direction in Y: +1 or -1.
    step_y: i32,
    /// Distance from current position to the next X grid boundary.
    side_dist_x: Fixed16_16,
    /// Distance from current position to the next Y grid boundary.
    side_dist_y: Fixed16_16,
    /// Distance between consecutive X grid boundaries along the ray.
    delta_dist_x: Fixed16_16,
    /// Distance between consecutive Y grid boundaries along the ray.
    delta_dist_y: Fixed16_16,
    /// The side of the last cell face that was hit.
    last_side: Side,
}

impl DdaStepper {
    /// Create a new DDA stepper from a world-space origin and a BAM angle.
    ///
    /// Uses `angle.sin_cos_fixed()` for a single table lookup. The ray
    /// direction is derived from cos (X component) and sin (Y component).
    #[must_use]
    pub fn new(origin: Vec2Fixed, angle: Bam) -> Self {
        // sin_cos_fixed returns (sin, cos) -- sin is FIRST
        let (sin, cos) = angle.sin_cos_fixed();

        // Delta distances: how far along the ray to cross one full grid cell
        // in each axis. delta = |1 / component|. If the component is zero,
        // the ray is parallel to that axis and will never cross it.
        let delta_dist_x = if cos.raw() == 0 {
            FIXED_INFINITY
        } else {
            FIXED_ONE.fixed_div(cos.abs())
        };

        let delta_dist_y = if sin.raw() == 0 {
            FIXED_INFINITY
        } else {
            FIXED_ONE.fixed_div(sin.abs())
        };

        // Starting cell coordinates (integer part of position).
        let cell_x = origin.x.to_int();
        let cell_y = origin.y.to_int();

        // Fractional position within the cell (0..1 in fixed-point).
        let frac_x = Fixed16_16::from_raw(origin.x.raw() & 0xFFFF);
        let frac_y = Fixed16_16::from_raw(origin.y.raw() & 0xFFFF);

        // Step direction and initial side distances.
        // side_dist = distance from origin to the first grid line in each axis.
        let (step_x, side_dist_x) = if cos.raw() >= 0 {
            // Ray points in +X direction (East)
            // Distance to the right edge of the current cell
            (1, (FIXED_ONE - frac_x).fixed_mul(delta_dist_x))
        } else {
            // Ray points in -X direction (West)
            // Distance to the left edge of the current cell
            (-1, frac_x.fixed_mul(delta_dist_x))
        };

        let (step_y, side_dist_y) = if sin.raw() >= 0 {
            // Ray points in +Y direction (North)
            // Distance to the top edge of the current cell
            (1, (FIXED_ONE - frac_y).fixed_mul(delta_dist_y))
        } else {
            // Ray points in -Y direction (South)
            // Distance to the bottom edge of the current cell
            (-1, frac_y.fixed_mul(delta_dist_y))
        };

        // Default to West since the first step hasn't happened yet.
        // This will be overwritten by the first call to step().
        let last_side = Side::West;

        Self {
            cell_x,
            cell_y,
            step_x,
            step_y,
            side_dist_x,
            side_dist_y,
            delta_dist_x,
            delta_dist_y,
            last_side,
        }
    }

    /// Advance to the next grid cell boundary.
    ///
    /// Returns the new cell coordinates and which face of the cell was entered.
    /// When stepping in +X, the ray enters via the West face. When stepping in
    /// +Y, the ray enters via the South face. And vice versa.
    pub fn step(&mut self) -> (i32, i32, Side) {
        if self.side_dist_x < self.side_dist_y {
            // Step in X
            self.side_dist_x += self.delta_dist_x;
            self.cell_x += self.step_x;
            self.last_side = if self.step_x > 0 {
                Side::West // Entered from the west (ray going east)
            } else {
                Side::East // Entered from the east (ray going west)
            };
        } else {
            // Step in Y
            self.side_dist_y += self.delta_dist_y;
            self.cell_y += self.step_y;
            self.last_side = if self.step_y > 0 {
                Side::South // Entered from the south (ray going north)
            } else {
                Side::North // Entered from the north (ray going south)
            };
        }

        (self.cell_x, self.cell_y, self.last_side)
    }

    /// Perpendicular distance from the ray origin to the last hit face.
    ///
    /// Computed as `side_dist - delta_dist` for the last-stepped axis. This
    /// gives the perpendicular (fisheye-corrected) distance for free from
    /// the DDA structure.
    #[must_use]
    pub fn perp_distance(&self) -> Fixed16_16 {
        match self.last_side {
            Side::West | Side::East => self.side_dist_x - self.delta_dist_x,
            Side::North | Side::South => self.side_dist_y - self.delta_dist_y,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests (TDD — written first)
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::bam::{ANG90, ANG180, ANG270};

    /// Helper: create a `Vec2Fixed` from f32 pair for readability.
    fn pos(x: f32, y: f32) -> Vec2Fixed {
        Vec2Fixed::from_f32(x, y)
    }

    #[test]
    fn step_east_along_x_axis() {
        // Ray at angle 0 (East) from center of cell (0,0).
        // Should step through (1,0), (2,0), (3,0)... hitting West faces.
        let mut dda = DdaStepper::new(pos(0.5, 0.5), Bam::ZERO);

        let (cx, cy, side) = dda.step();
        assert_eq!((cx, cy), (1, 0), "first step should be cell (1,0)");
        assert_eq!(side, Side::West, "entering from the west");

        let (cx, cy, side) = dda.step();
        assert_eq!((cx, cy), (2, 0), "second step should be cell (2,0)");
        assert_eq!(side, Side::West);

        let (cx, cy, side) = dda.step();
        assert_eq!((cx, cy), (3, 0), "third step should be cell (3,0)");
        assert_eq!(side, Side::West);
    }

    #[test]
    fn step_north_along_y_axis() {
        // Ray at 90 degrees (North/+Y) from center of cell (0,0).
        // Should step through (0,1), (0,2)... hitting South faces.
        let mut dda = DdaStepper::new(pos(0.5, 0.5), ANG90);

        let (cx, cy, side) = dda.step();
        assert_eq!((cx, cy), (0, 1), "first step should be cell (0,1)");
        assert_eq!(side, Side::South, "entering from the south");

        let (cx, cy, side) = dda.step();
        assert_eq!((cx, cy), (0, 2), "second step should be cell (0,2)");
        assert_eq!(side, Side::South);
    }

    #[test]
    fn step_west() {
        // Ray at 180 degrees (West/-X) from (1.5, 0.5).
        // Should step to (0,0) hitting East face.
        let mut dda = DdaStepper::new(pos(1.5, 0.5), ANG180);

        let (cx, cy, side) = dda.step();
        assert_eq!((cx, cy), (0, 0), "should step to cell (0,0)");
        assert_eq!(side, Side::East, "entering from the east");
    }

    #[test]
    fn step_south() {
        // Ray at 270 degrees (South/-Y) from (0.5, 1.5).
        // Should step to (0,0) hitting North face.
        let mut dda = DdaStepper::new(pos(0.5, 1.5), ANG270);

        let (cx, cy, side) = dda.step();
        assert_eq!((cx, cy), (0, 0), "should step to cell (0,0)");
        assert_eq!(side, Side::North, "entering from the north");
    }

    #[test]
    fn perp_distance_east() {
        // From (0.5, 0.5) facing East, the first grid line is at x=1.0.
        // Perpendicular distance should be ~0.5.
        let mut dda = DdaStepper::new(pos(0.5, 0.5), Bam::ZERO);
        dda.step();

        let dist = dda.perp_distance();
        let dist_f32 = dist.to_f32();
        assert!(
            (dist_f32 - 0.5).abs() < 0.01,
            "perp distance should be ~0.5, got {dist_f32}"
        );
    }

    #[test]
    fn multiple_steps_increase_distance() {
        // Each step east should increase the perpendicular distance by ~1.0.
        let mut dda = DdaStepper::new(pos(0.5, 0.5), Bam::ZERO);

        let mut prev_dist = Fixed16_16::ZERO;
        for i in 0..5 {
            dda.step();
            let dist = dda.perp_distance();
            assert!(
                dist > prev_dist,
                "step {i}: distance {dist} should be > previous {prev_dist}"
            );
            prev_dist = dist;
        }
    }

    #[test]
    fn diagonal_ray_alternates_axes() {
        // A 45-degree ray should alternate between X and Y steps.
        use abrash_core::bam::ANG45;

        let mut dda = DdaStepper::new(pos(0.5, 0.5), ANG45);

        // From the center of a cell at 45 degrees, both side distances
        // should be equal, so the stepper should alternate. The exact
        // order depends on the tie-breaking rule (we use < so ties go to Y).
        let mut x_steps: i32 = 0;
        let mut y_steps: i32 = 0;
        for _ in 0..10 {
            let (_, _, side) = dda.step();
            match side {
                Side::West | Side::East => x_steps += 1,
                Side::North | Side::South => y_steps += 1,
            }
        }
        // Both axes should get roughly equal steps for a 45-degree ray.
        assert!(
            (x_steps - y_steps).abs() <= 1,
            "45-degree ray should alternate: x_steps={x_steps}, y_steps={y_steps}"
        );
    }

    #[test]
    fn initial_cell_matches_origin() {
        // The stepper should start at the cell containing the origin.
        let dda = DdaStepper::new(pos(3.7, 5.2), Bam::ZERO);
        assert_eq!(dda.cell_x, 3);
        assert_eq!(dda.cell_y, 5);
    }

    #[test]
    fn perp_distance_north() {
        // From (0.5, 0.5) facing North, the first grid line is at y=1.0.
        // Perpendicular distance should be ~0.5.
        let mut dda = DdaStepper::new(pos(0.5, 0.5), ANG90);
        dda.step();

        let dist = dda.perp_distance();
        let dist_f32 = dist.to_f32();
        assert!(
            (dist_f32 - 0.5).abs() < 0.01,
            "perp distance north should be ~0.5, got {dist_f32}"
        );
    }

    #[test]
    fn perp_distance_increases_by_one_per_step() {
        // Facing east from center, each step adds exactly 1.0 to perp distance.
        let mut dda = DdaStepper::new(pos(0.5, 0.5), Bam::ZERO);

        for expected in 1..=5 {
            dda.step();
            let dist = dda.perp_distance();
            let dist_f32 = dist.to_f32();
            let expected_f32 = expected as f32 - 0.5;
            assert!(
                (dist_f32 - expected_f32).abs() < 0.01,
                "step {expected}: perp distance should be ~{expected_f32}, got {dist_f32}"
            );
        }
    }
}
