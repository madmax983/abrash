//! Experimental Texture Mapping module.
//!
//! Implements affine texture mapping for the software rasterizer.

use crate::framebuffer::Framebuffer;
use crate::math::{Vec2, Vec3, project_to_screen};
use crate::mesh::Mesh;
use crate::zbuffer::ZBuffer;

/// A simple 2D texture
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

impl Texture {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0xFF000000; (width * height) as usize],
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = color;
        }
    }

    /// Sample texture using nearest neighbor interpolation
    /// u, v are in range [0.0, 1.0]
    pub fn get_pixel(&self, u: f32, v: f32) -> u32 {
        let x = (u * self.width as f32) as i32;
        let y = (v * self.height as f32) as i32;
        self.get_pixel_texel(x, y)
    }

    /// Sample texture using texel coordinates
    #[inline(always)]
    pub fn get_pixel_texel(&self, x: i32, y: i32) -> u32 {
        // Clamp to edge
        let x = x.clamp(0, self.width as i32 - 1) as usize;
        let y = y.clamp(0, self.height as i32 - 1) as usize;
        // unsafe { *self.pixels.get_unchecked(y * self.width as usize + x) }
        self.pixels[y * self.width as usize + x]
    }

    /// Create a checkerboard texture
    pub fn checkered(width: u32, height: u32, c1: u32, c2: u32) -> Self {
        let mut tex = Self::new(width, height);
        // Scale checks based on size, defaulting to 8x8 blocks
        let block_w = (width / 8).max(1);
        let block_h = (height / 8).max(1);

        for y in 0..height {
            for x in 0..width {
                let check = ((x / block_w) + (y / block_h)) & 1 == 0;
                tex.set_pixel(x, y, if check { c1 } else { c2 });
            }
        }
        tex
    }
}

/// A wrapper around Mesh that includes UV coordinates
pub struct TexturedMesh {
    pub mesh: Mesh,
    pub uvs: Vec<Vec2>,
}

impl TexturedMesh {
    /// Create a textured cube with duplicated vertices for proper UV mapping
    pub fn textured_cube(size: f32) -> Self {
        let h = size / 2.0;
        let mut vertices = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        // Helper to add a quad
        // v0: BL, v1: BR, v2: TR, v3: TL
        let mut add_quad = |v0, v1, v2, v3| {
            let base = vertices.len();
            vertices.push(v0);
            vertices.push(v1);
            vertices.push(v2);
            vertices.push(v3);

            // Standard UV mapping (0,0 bottom-left, 1,1 top-right)
            uvs.push(Vec2::new(0.0, 0.0));
            uvs.push(Vec2::new(1.0, 0.0));
            uvs.push(Vec2::new(1.0, 1.0));
            uvs.push(Vec2::new(0.0, 1.0));

            indices.push([base, base + 1, base + 2]);
            indices.push([base, base + 2, base + 3]);
        };

        // Front Face (+Z)
        add_quad(
            Vec3::new(-h, -h, h),
            Vec3::new(h, -h, h),
            Vec3::new(h, h, h),
            Vec3::new(-h, h, h),
        );

        // Back Face (-Z)
        add_quad(
            Vec3::new(h, -h, -h),  // 5: BL (from back view)
            Vec3::new(-h, -h, -h), // 4: BR
            Vec3::new(-h, h, -h),  // 7: TR
            Vec3::new(h, h, -h),   // 6: TL
        );

        // Top Face (+Y)
        add_quad(
            Vec3::new(-h, h, h),  // 3: BL
            Vec3::new(h, h, h),   // 2: BR
            Vec3::new(h, h, -h),  // 6: TR
            Vec3::new(-h, h, -h), // 7: TL
        );

        // Bottom Face (-Y)
        add_quad(
            Vec3::new(-h, -h, -h), // 4: BL
            Vec3::new(h, -h, -h),  // 5: BR
            Vec3::new(h, -h, h),   // 1: TR
            Vec3::new(-h, -h, h),  // 0: TL
        );

        // Right Face (+X)
        add_quad(
            Vec3::new(h, -h, h),  // 1: BL
            Vec3::new(h, -h, -h), // 5: BR
            Vec3::new(h, h, -h),  // 6: TR
            Vec3::new(h, h, h),   // 2: TL
        );

        // Left Face (-X)
        add_quad(
            Vec3::new(-h, -h, -h), // 4: BL
            Vec3::new(-h, -h, h),  // 0: BR
            Vec3::new(-h, h, h),   // 3: TR
            Vec3::new(-h, h, -h),  // 7: TL
        );

        let mesh = Mesh { vertices, indices };

        Self { mesh, uvs }
    }
}

/// Helper to sort 3 vertices by Y coordinate
fn sort_by_y<T, F>(verts: &mut [T; 3], get_y: F)
where
    F: Fn(&T) -> i32,
{
    if get_y(&verts[0]) > get_y(&verts[1]) {
        verts.swap(0, 1);
    }
    if get_y(&verts[1]) > get_y(&verts[2]) {
        verts.swap(1, 2);
    }
    if get_y(&verts[0]) > get_y(&verts[1]) {
        verts.swap(0, 1);
    }
}

fn assert_same_dimensions(fb: &Framebuffer, zb: &ZBuffer) {
    assert_eq!(
        fb.width(),
        zb.width(),
        "Framebuffer and ZBuffer widths must match"
    );
    assert_eq!(
        fb.height(),
        zb.height(),
        "Framebuffer and ZBuffer heights must match"
    );
}

/// Draw a single scanline with texture mapping
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn draw_scanline_textured(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    texture: &Texture,
    y: i32,
    x_start: i32,
    x_end: i32,
    z_start: f32,
    uv_start: Vec2,
    dz_dx: f32,
    duv_dx: (i32, i32), // Fixed point gradients
) {
    let width = fb.width() as i32;
    let mut xs = x_start;
    let mut xe = x_end;
    let mut z = z_start;

    // Fixed point 16.16 setup
    const SCALE: f32 = 65536.0;

    // Convert start UVs to Texels first!
    // We want to interpolate Texel coordinates (0..width, 0..height)
    // uv_start is already in Texel coordinates (pre-scaled in fill_triangle)
    let mut u_i = (uv_start.x * SCALE) as i64;
    let mut v_i = (uv_start.y * SCALE) as i64;

    let (du, dv) = (duv_dx.0 as i64, duv_dx.1 as i64);

    if xs < 0 {
        let diff = -xs;
        z += (diff as f32) * dz_dx;
        let diff_i64 = diff as i64;
        u_i += diff_i64 * du;
        v_i += diff_i64 * dv;
        xs = 0;
    }

    if xe >= width {
        xe = width - 1;
    }

    // Demote to i32 for hot loop
    let mut u_i = u_i as i32;
    let mut v_i = v_i as i32;
    let du = du as i32;
    let dv = dv as i32;

    if xs <= xe {
        let width_usize = fb.width() as usize;
        let y_offset = (y as usize) * width_usize;
        let start_idx = y_offset + (xs as usize);
        let end_idx = y_offset + (xe as usize);

        let fb_slice = unsafe { fb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };
        let zb_slice = unsafe { zb.as_mut_slice().get_unchecked_mut(start_idx..=end_idx) };

        for (pixel, depth_val) in fb_slice.iter_mut().zip(zb_slice.iter_mut()) {
            if z < *depth_val {
                *depth_val = z;
                // Sample texture
                let tx = u_i >> 16;
                let ty = v_i >> 16;
                *pixel = texture.get_pixel_texel(tx, ty);
            }
            z += dz_dx;
            u_i += du;
            v_i += dv;
        }
    }
}

/// Fill a 3D triangle with affine texture mapping
pub fn fill_triangle_textured(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    v0: ((Vec3, f32), Vec2), // (Position, W), UV
    v1: ((Vec3, f32), Vec2),
    v2: ((Vec3, f32), Vec2),
    texture: &Texture,
) {
    assert_same_dimensions(fb, zb);

    let width = fb.width();
    let height = fb.height();

    // Project to screen
    let p0 = project_to_screen(v0.0.0, v0.0.1, width, height);
    let p1 = project_to_screen(v1.0.0, v1.0.1, width, height);
    let p2 = project_to_screen(v2.0.0, v2.0.1, width, height);

    // Scale UVs to Texture Dimensions for easier interpolation
    let uv0 = Vec2::new(
        v0.1.x * texture.width as f32,
        v0.1.y * texture.height as f32,
    );
    let uv1 = Vec2::new(
        v1.1.x * texture.width as f32,
        v1.1.y * texture.height as f32,
    );
    let uv2 = Vec2::new(
        v2.1.x * texture.width as f32,
        v2.1.y * texture.height as f32,
    );

    let mut verts = [(p0, uv0), (p1, uv1), (p2, uv2)];
    sort_by_y(&mut verts, |(p, _)| p.y);
    let [(p0, uv0), (p1, uv1), (p2, uv2)] = verts;

    let total_height = (p2.y as i64 - p0.y as i64) as f32;
    if total_height == 0.0 {
        return;
    }

    let y_min = 0;
    let y_max = height as i32 - 1;
    let y_start = p0.y.max(y_min);
    let y_end = p2.y.min(y_max);

    if y_start > y_end {
        return;
    }

    // Gradients
    let ux = (p1.x as i64 - p0.x as i64) as f32;
    let uy = (p1.y as i64 - p0.y as i64) as f32;
    let uz = p1.z - p0.z;
    let u_uv = uv1 - uv0;

    let vx = (p2.x as i64 - p0.x as i64) as f32;
    let vy = (p2.y as i64 - p0.y as i64) as f32;
    let vz = p2.z - p0.z;
    let v_uv = uv2 - uv0;

    // Cross product Z (area)
    let nz = ux * vy - uy * vx;
    let long_edge_is_left = nz > 0.0;
    let inv_nz = if nz.abs() > 0.0001 { -1.0 / nz } else { 0.0 };

    // dz/dx
    let nx_z = uy * vz - uz * vy;
    let dz_dx = nx_z * inv_nz;

    // duv/dx
    let nx_u = uy * v_uv.x - u_uv.x * vy;
    let nx_v = uy * v_uv.y - u_uv.y * vy;

    let du = nx_u * inv_nz;
    let dv = nx_v * inv_nz;

    // Convert to fixed point 16.16
    const SCALE: f32 = 65536.0;
    let du_i = (du * SCALE) as i32;
    let dv_i = (dv * SCALE) as i32;
    let duv_dx_int = (du_i, dv_i);

    // Edge gradients
    let inv_total_height = 1.0 / total_height;
    let dx_dy_a = (p2.x - p0.x) as f32 * inv_total_height;
    let dz_dy_a = (p2.z - p0.z) * inv_total_height;
    let duv_dy_a = (uv2 - uv0) * inv_total_height;

    let mut ax = p0.x as f32;
    let mut az = p0.z;
    let mut auv = uv0;

    let mut bx = p0.x as f32;
    let mut bz = p0.z;
    let mut buv = uv0;

    let h1 = (p1.y - p0.y) as f32;
    let (dx_dy_b1, dz_dy_b1, duv_dy_b1) = if h1 != 0.0 {
        let inv_h1 = 1.0 / h1;
        (
            (p1.x - p0.x) as f32 * inv_h1,
            (p1.z - p0.z) * inv_h1,
            (uv1 - uv0) * inv_h1,
        )
    } else {
        (0.0, 0.0, Vec2::default())
    };

    if y_start > p0.y {
        let dy = (y_start - p0.y) as f32;
        ax += dx_dy_a * dy;
        az += dz_dy_a * dy;
        auv = auv + duv_dy_a * dy;

        if y_start < p1.y {
            bx += dx_dy_b1 * dy;
            bz += dz_dy_b1 * dy;
            buv = buv + duv_dy_b1 * dy;
        } else {
            bx = p1.x as f32;
            bz = p1.z;
            buv = uv1;

            let h2 = (p2.y - p1.y) as f32;
            if h2 != 0.0 {
                let inv_h2 = 1.0 / h2;
                let dx_dy_b2 = (p2.x - p1.x) as f32 * inv_h2;
                let dz_dy_b2 = (p2.z - p1.z) * inv_h2;
                let duv_dy_b2 = (uv2 - uv1) * inv_h2;

                let dy2 = (y_start - p1.y) as f32;
                bx += dx_dy_b2 * dy2;
                bz += dz_dy_b2 * dy2;
                buv = buv + duv_dy_b2 * dy2;
            }
        }
    }

    let mut dx_dy_b = dx_dy_b1;
    let mut dz_dy_b = dz_dy_b1;
    let mut duv_dy_b = duv_dy_b1;

    if y_start >= p1.y {
        let h2 = (p2.y - p1.y) as f32;
        if h2 != 0.0 {
            let inv_h2 = 1.0 / h2;
            dx_dy_b = (p2.x - p1.x) as f32 * inv_h2;
            dz_dy_b = (p2.z - p1.z) * inv_h2;
            duv_dy_b = (uv2 - uv1) * inv_h2;
        }
    }

    for y in y_start..=y_end {
        if y == p1.y && y != p0.y {
            bx = p1.x as f32;
            bz = p1.z;
            buv = uv1;
            let h2 = (p2.y - p1.y) as f32;
            if h2 != 0.0 {
                let inv_h2 = 1.0 / h2;
                dx_dy_b = (p2.x - p1.x) as f32 * inv_h2;
                dz_dy_b = (p2.z - p1.z) * inv_h2;
                duv_dy_b = (uv2 - uv1) * inv_h2;
            }
        }

        let (x_left, z_left, uv_left, x_right, _z_right, _uv_right) = if long_edge_is_left {
            (ax, az, auv, bx, bz, buv)
        } else {
            (bx, bz, buv, ax, az, auv)
        };

        let x_start = x_left as i32;
        let x_end = x_right as i32;
        let dx = x_end - x_start;

        if dx <= 0 {
            if x_start >= 0 && x_start < width as i32 && zb.test_and_set(x_start, y, z_left) {
                // Convert tex coords for single pixel
                let tx = (uv_left.x + 0.5) as i32;
                let ty = (uv_left.y + 0.5) as i32;
                fb.set_pixel(x_start, y, texture.get_pixel_texel(tx, ty));
            }
        } else {
            draw_scanline_textured(
                fb, zb, texture, y, x_start, x_end, z_left, uv_left, dz_dx, duv_dx_int,
            );
        }

        ax += dx_dy_a;
        az += dz_dy_a;
        auv = auv + duv_dy_a;

        bx += dx_dy_b;
        bz += dz_dy_b;
        buv = buv + duv_dy_b;
    }
}
