use crate::math::{ScreenPoint, Vec3};
use crate::rasterizer::scanline::FIXED_SCALE;

pub(crate) struct EdgeWalker {
    pub(crate) x: f32,
    pub(crate) z: f32,
    pub(crate) dx_dy: f32,
    pub(crate) dz_dy: f32,
}

impl EdgeWalker {
    pub(crate) fn new(p_start: ScreenPoint, p_end: ScreenPoint) -> Self {
        let height = (p_end.y as i64 - p_start.y as i64) as f32;
        let (dx_dy, dz_dy) = if height != 0.0 {
            let inv_h = 1.0 / height;
            (
                (p_end.x as i64 - p_start.x as i64) as f32 * inv_h,
                (p_end.z - p_start.z) * inv_h,
            )
        } else {
            (0.0, 0.0)
        };

        Self {
            x: p_start.x as f32,
            z: p_start.z,
            dx_dy,
            dz_dy,
        }
    }

    pub(crate) fn step(&mut self) {
        self.x += self.dx_dy;
        self.z += self.dz_dy;
    }

    pub(crate) fn step_n(&mut self, n: i32) {
        let n_f = n as f32;
        self.x += self.dx_dy * n_f;
        self.z += self.dz_dy * n_f;
    }
}

pub(crate) struct GouraudGradients {
    pub(crate) dz_dx: f32,
    pub(crate) dc_dx: (i32, i32, i32),
}

impl GouraudGradients {
    pub(crate) fn new(
        p0: ScreenPoint,
        p1: ScreenPoint,
        p2: ScreenPoint,
        c0: Vec3,
        c1: Vec3,
        c2: Vec3,
    ) -> Self {
        let ux = (p1.x as i64 - p0.x as i64) as f32;
        let uy = (p1.y as i64 - p0.y as i64) as f32;
        let uz = p1.z - p0.z;
        let uc = c1 - c0;

        let vx = (p2.x as i64 - p0.x as i64) as f32;
        let vy = (p2.y as i64 - p0.y as i64) as f32;
        let vz = p2.z - p0.z;
        let vc = c2 - c0;

        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;

        let nx_r = uy * vc.x - uc.x * vy;
        let nx_g = uy * vc.y - uc.y * vy;
        let nx_b = uy * vc.z - uc.z * vy;

        let dr = nx_r * inv_nz;
        let dg = nx_g * inv_nz;
        let db = nx_b * inv_nz;

        let dr_i = (dr * FIXED_SCALE) as i32;
        let dg_i = (dg * FIXED_SCALE) as i32;
        let db_i = (db * FIXED_SCALE) as i32;

        Self {
            dz_dx,
            dc_dx: (dr_i, dg_i, db_i),
        }
    }

    pub(crate) fn is_long_edge_left(p0: ScreenPoint, p1: ScreenPoint, p2: ScreenPoint) -> bool {
        let ux = (p1.x as i64 - p0.x as i64) as f32;
        let uy = (p1.y as i64 - p0.y as i64) as f32;
        let vx = (p2.x as i64 - p0.x as i64) as f32;
        let vy = (p2.y as i64 - p0.y as i64) as f32;
        ux * vy - uy * vx > 0.0
    }
}

pub(crate) struct GouraudEdgeWalker {
    pub(crate) x: f32,
    pub(crate) z: f32,
    pub(crate) c: (i64, i64, i64),
    pub(crate) dx_dy: f32,
    pub(crate) dz_dy: f32,
    pub(crate) dc_dy: (i64, i64, i64),
}

impl GouraudEdgeWalker {
    pub(crate) fn new(
        p_start: ScreenPoint,
        p_end: ScreenPoint,
        c_start: Vec3,
        c_end: Vec3,
    ) -> Self {
        let height = (p_end.y as i64 - p_start.y as i64) as f32;
        let (dx_dy, dz_dy, dc_dy) = if height != 0.0 {
            let inv_h = 1.0 / height;
            let dc = (c_end - c_start) * inv_h;
            (
                (p_end.x as i64 - p_start.x as i64) as f32 * inv_h,
                (p_end.z - p_start.z) * inv_h,
                (
                    (dc.x * FIXED_SCALE) as i64,
                    (dc.y * FIXED_SCALE) as i64,
                    (dc.z * FIXED_SCALE) as i64,
                ),
            )
        } else {
            (0.0, 0.0, (0, 0, 0))
        };

        let c_fixed = (
            (c_start.x * FIXED_SCALE) as i64,
            (c_start.y * FIXED_SCALE) as i64,
            (c_start.z * FIXED_SCALE) as i64,
        );

        Self {
            x: p_start.x as f32,
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

    pub(crate) fn step_n(&mut self, n: i32) {
        let n_f = n as f32;
        let n_i64 = n as i64;
        self.x += self.dx_dy * n_f;
        self.z += self.dz_dy * n_f;
        self.c.0 += self.dc_dy.0 * n_i64;
        self.c.1 += self.dc_dy.1 * n_i64;
        self.c.2 += self.dc_dy.2 * n_i64;
    }
}
