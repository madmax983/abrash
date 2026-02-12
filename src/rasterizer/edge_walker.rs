use crate::math::{ScreenPoint, Vec3};
use super::FIXED_SCALE;

pub(crate) struct EdgeWalker {
    pub(crate) x: i64,
    dx_dy: i64,
    pub(crate) z: f32,
    dz_dy: f32,
}

impl EdgeWalker {
    pub(crate) fn new(p_start: ScreenPoint, p_end: ScreenPoint) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let (dx_dy, dz_dy) = if height == 0.0 {
            (0, 0.0)
        } else {
            let inv_h = 1.0 / height;

            (
                ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64,
                (p_end.z - p_start.z) * inv_h,
            )
        };

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            dx_dy,
            dz_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * (n as f32);
    }
}

pub(crate) struct GouraudEdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: f32,
    pub(crate) c: (i64, i64, i64),
    dx_dy: i64,
    dz_dy: f32,
    dc_dy: (i64, i64, i64),
}

impl GouraudEdgeWalker {
    pub(crate) fn new(p_start: ScreenPoint, p_end: ScreenPoint, c_start: Vec3, c_end: Vec3) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let (dx_dy, dz_dy, dc_dy) = if height == 0.0 {
            (0, 0.0, (0, 0, 0))
        } else {
            let inv_h = 1.0 / height;
            let dc = (c_end - c_start) * inv_h;
            (
                ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64,
                (p_end.z - p_start.z) * inv_h,
                (
                    (dc.x * FIXED_SCALE) as i64,
                    (dc.y * FIXED_SCALE) as i64,
                    (dc.z * FIXED_SCALE) as i64,
                ),
            )
        };

        let c_fixed = (
            (c_start.x * FIXED_SCALE) as i64,
            (c_start.y * FIXED_SCALE) as i64,
            (c_start.z * FIXED_SCALE) as i64,
        );

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            c: c_fixed,
            dx_dy,
            dz_dy,
            dc_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.c.0 += self.dc_dy.0;
        self.c.1 += self.dc_dy.1;
        self.c.2 += self.dc_dy.2;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.c.0 = self.c.0.wrapping_add(self.dc_dy.0.wrapping_mul(n));
        self.c.1 = self.c.1.wrapping_add(self.dc_dy.1.wrapping_mul(n));
        self.c.2 = self.c.2.wrapping_add(self.dc_dy.2.wrapping_mul(n));
    }
}

pub(crate) struct PerspectiveTextureEdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: f32,
    pub(crate) q: f32, // 1/w
    pub(crate) u: f32, // u/w
    pub(crate) v: f32, // v/w
    dx_dy: i64,
    dz_dy: f32,
    dq_dy: f32,
    du_dy: f32,
    dv_dy: f32,
}

impl PerspectiveTextureEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        q_start: f32,
        q_end: f32,
        u_start: f32,
        u_end: f32,
        v_start: f32,
        v_end: f32,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dq_dy = (q_end - q_start) * inv_h;
        let du_dy = (u_end - u_start) * inv_h;
        let dv_dy = (v_end - v_start) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            q: q_start,
            u: u_start,
            v: v_start,
            dx_dy,
            dz_dy,
            dq_dy,
            du_dy,
            dv_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.q += self.dq_dy;
        self.u += self.du_dy;
        self.v += self.dv_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.q += self.dq_dy * n_f;
        self.u += self.du_dy * n_f;
        self.v += self.dv_dy * n_f;
    }
}

pub(crate) struct PhongEdgeWalker {
    pub(crate) x: i64,
    pub(crate) z: f32,
    pub(crate) nx: f32,
    pub(crate) ny: f32,
    pub(crate) nz: f32,
    dx_dy: i64,
    dz_dy: f32,
    dnx_dy: f32,
    dny_dy: f32,
    dnz_dy: f32,
}

impl PhongEdgeWalker {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        n_start: Vec3,
        n_end: Vec3,
    ) -> Self {
        let height = (i64::from(p_end.y) - i64::from(p_start.y)) as f32;
        let inv_h = if height == 0.0 { 0.0 } else { 1.0 / height };

        let dx_dy =
            ((i64::from(p_end.x) - i64::from(p_start.x)) as f32 * inv_h * FIXED_SCALE) as i64;
        let dz_dy = (p_end.z - p_start.z) * inv_h;
        let dnx_dy = (n_end.x - n_start.x) * inv_h;
        let dny_dy = (n_end.y - n_start.y) * inv_h;
        let dnz_dy = (n_end.z - n_start.z) * inv_h;

        Self {
            x: i64::from(p_start.x) << 16,
            z: p_start.z,
            nx: n_start.x,
            ny: n_start.y,
            nz: n_start.z,
            dx_dy,
            dz_dy,
            dnx_dy,
            dny_dy,
            dnz_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
        self.nx += self.dnx_dy;
        self.ny += self.dny_dy;
        self.nz += self.dnz_dy;
    }

    pub(crate) fn step_n(&mut self, n: i64) {
        let n_f = n as f32;
        self.x = self.x.wrapping_add(self.dx_dy.wrapping_mul(n));
        self.z += self.dz_dy * n_f;
        self.nx += self.dnx_dy * n_f;
        self.ny += self.dny_dy * n_f;
        self.nz += self.dnz_dy * n_f;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::ScreenPoint;

    #[test]
    fn edge_walker_z_interpolation() {
        // Test that EdgeWalker correctly interpolates z
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let walker = EdgeWalker::new(p0, p1);

        // z should be 1.0
        assert!((walker.z - 1.0).abs() < 0.0001);

        // dz_dy should be (2.0 - 1.0) / 100.0 = 0.01
        let expected_dz_dy = (2.0_f32 - 1.0_f32) / 100.0_f32;
        assert!((walker.dz_dy - expected_dz_dy).abs() < 0.0001);
    }

    #[test]
    fn edge_walker_step_accumulates_correctly() {
        // Test that stepping accumulates z correctly
        let p0 = ScreenPoint {
            x: 0,
            y: 0,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let mut walker = EdgeWalker::new(p0, p1);

        let initial_z = walker.z;
        let dz = walker.dz_dy;

        // Step 50 times
        walker.step_n(50);

        // After 50 steps, z should be initial + 50*dz
        let expected_z = initial_z + dz * 50.0;
        assert!((walker.z - expected_z).abs() < 0.0001);

        assert!(
            walker.z > 1.0 && walker.z < 2.0,
            "z should be interpolated between 1.0 and 2.0, got {}",
            walker.z,
        );
    }

    #[test]
    fn edge_walker_zero_height_no_panic() {
        // Test that EdgeWalker handles degenerate case (zero height)
        let p0 = ScreenPoint {
            x: 0,
            y: 100,
            z: 1.0,
            inv_w: 1.0,
        };
        let p1 = ScreenPoint {
            x: 100,
            y: 100,
            z: 2.0,
            inv_w: 1.0,
        };

        let walker = EdgeWalker::new(p0, p1);

        // Should have zero gradients
        assert_eq!(walker.dx_dy, 0);
        assert_eq!(walker.dz_dy, 0.0);

        // z should still be correct
        assert!((walker.z - 1.0).abs() < 0.0001);
    }
}
