import re

with open("crates/abrash-render/src/rasterizer/core.rs", "r") as f:
    content = f.read()

new_nz = """
pub(crate) struct BaseTriangleSetup {
    pub ux: f32,
    pub uy: f32,
    pub vx: f32,
    pub vy: f32,
    pub nz: f32,
    pub inv_nz: f32,
    pub dz_dx: f32,
}

impl BaseTriangleSetup {
    #[inline(always)]
    pub(crate) fn compute(p0: ScreenPoint, p1: ScreenPoint, p2: ScreenPoint) -> Self {
        let ux = (i64::from(p1.x) - i64::from(p0.x)) as f32;
        let uy = (i64::from(p1.y) - i64::from(p0.y)) as f32;
        let uz = p1.z - p0.z;
        let vx = (i64::from(p2.x) - i64::from(p0.x)) as f32;
        let vy = (i64::from(p2.y) - i64::from(p0.y)) as f32;
        let vz = p2.z - p0.z;
        let nz = ux * vy - uy * vx;
        let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };
        let nx_z = uy * vz - uz * vy;
        let dz_dx = nx_z * inv_nz;
        Self {
            ux,
            uy,
            vx,
            vy,
            nz,
            inv_nz,
            dz_dx,
        }
    }
"""

content = re.sub(r'pub\(crate\) struct BaseTriangleSetup \{\s*pub ux: f32,\s*pub uy: f32,\s*pub vx: f32,\s*pub vy: f32,\s*pub inv_nz: f32,\s*pub dz_dx: f32,\s*\}\s*impl BaseTriangleSetup \{\s*#\[inline\(always\)\]\s*pub\(crate\) fn compute[^{]*\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\}\s*', new_nz.strip() + "\n\n", content, count=1)

with open("crates/abrash-render/src/rasterizer/core.rs", "w") as f:
    f.write(content)
