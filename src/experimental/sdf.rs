//! Signed Distance Field (SDF) Renderer
//!
//! A hybrid renderer that ray-marches SDF primitives and composites them
//! into the rasterized depth buffer. This allows for mathematically perfect
//! shapes (Spheres, Boxes) to interact with polygon meshes.

use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, Vec2, Vec3};
use crate::zbuffer::ZBuffer;

/// Supported SDF Primitives.
#[derive(Clone, Copy, Debug)]
pub enum SdfPrimitive {
    Sphere {
        radius: f32,
        center: Vec3,
    },
    Box {
        size: Vec3,
        center: Vec3,
    },
    Torus {
        major_radius: f32,
        minor_radius: f32,
        center: Vec3,
    },
    Plane {
        normal: Vec3,
        distance: f32,
    },
    Capsule {
        start: Vec3,
        end: Vec3,
        radius: f32,
    },
}

/// An object in the SDF scene.
#[derive(Clone, Copy, Debug)]
pub struct SdfObject {
    pub primitive: SdfPrimitive,
    pub color: u32,
}

impl SdfObject {
    /// Evaluate the signed distance to this object.
    #[must_use]
    pub fn distance(&self, p: Vec3) -> f32 {
        match self.primitive {
            SdfPrimitive::Sphere { radius, center } => (p - center).length() - radius,
            SdfPrimitive::Box { size, center } => {
                let d = vec3_abs(p - center) - size;
                let inside_dist = d.x.max(d.y).max(d.z).min(0.0);
                let outside_dist = vec3_max(d, 0.0).length();
                inside_dist + outside_dist
            }
            SdfPrimitive::Torus {
                major_radius,
                minor_radius,
                center,
            } => {
                let p = p - center;
                let q = Vec2::new(Vec2::new(p.x, p.z).length() - major_radius, p.y);
                q.length() - minor_radius
            }
            SdfPrimitive::Plane { normal, distance } => p.dot(normal) + distance,
            SdfPrimitive::Capsule { start, end, radius } => {
                let pa = p - start;
                let ba = end - start;
                let h = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
                (pa - ba * h).length() - radius
            }
        }
    }
}

// Helpers
fn vec3_abs(v: Vec3) -> Vec3 {
    Vec3::new(v.x.abs(), v.y.abs(), v.z.abs())
}

fn vec3_max(v: Vec3, val: f32) -> Vec3 {
    Vec3::new(v.x.max(val), v.y.max(val), v.z.max(val))
}

trait Vec2Ext {
    fn length(self) -> f32;
}

impl Vec2Ext for Vec2 {
    fn length(self) -> f32 {
        self.x.hypot(self.y)
    }
}

/// A scene containing SDF objects.
pub struct SdfScene {
    pub objects: Vec<SdfObject>,
}

impl Default for SdfScene {
    fn default() -> Self {
        Self::new()
    }
}

impl SdfScene {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    pub fn add(&mut self, object: SdfObject) {
        self.objects.push(object);
    }

    /// Find the minimum distance to the scene.
    /// Returns (distance, color).
    #[must_use]
    pub fn map(&self, p: Vec3) -> (f32, u32) {
        let mut min_dist = f32::MAX;
        let mut min_color = 0xFF000000;

        for obj in &self.objects {
            let d = obj.distance(p);
            if d < min_dist {
                min_dist = d;
                min_color = obj.color;
            }
        }
        (min_dist, min_color)
    }

    /// Calculate the normal at a point using finite differences.
    #[must_use]
    pub fn normal(&self, p: Vec3) -> Vec3 {
        let eps = 0.001;
        let d = self.map(p).0;
        let n = Vec3::new(
            self.map(Vec3::new(p.x + eps, p.y, p.z)).0 - d,
            self.map(Vec3::new(p.x, p.y + eps, p.z)).0 - d,
            self.map(Vec3::new(p.x, p.y, p.z + eps)).0 - d,
        );
        n.normalize()
    }
}

/// Renders the SDF scene into the framebuffer, respecting the Z-buffer.
///
/// # Arguments
///
/// * `fb` - Target Framebuffer.
/// * `zb` - Target Z-buffer (read/write).
/// * `view` - View Matrix (World -> Camera).
/// * `proj` - Projection Matrix.
/// * `camera_pos` - Camera position in World Space.
pub fn render_sdf(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    scene: &SdfScene,
    view: &Mat4,
    proj: &Mat4,
    camera_pos: Vec3,
) {
    let width = fb.width() as usize;
    let height = fb.height() as usize;

    // Extract Camera Basis from View Matrix (Row-Major)
    // Column 0 = Right (View.m[0][0], View.m[1][0], View.m[2][0])
    // Column 1 = Up    (View.m[0][1], View.m[1][1], View.m[2][1])
    // Column 2 = Back  (View.m[0][2], View.m[1][2], View.m[2][2]) -> Forward = -Back
    let right = Vec3::new(view.m[0][0], view.m[1][0], view.m[2][0]).normalize();
    let up = Vec3::new(view.m[0][1], view.m[1][1], view.m[2][1]).normalize();
    let forward = Vec3::new(-view.m[0][2], -view.m[1][2], -view.m[2][2]).normalize();

    // Calculate FoV factor
    // Projection Matrix [0][0] = 1 / (aspect * tan(fov/2))
    // Projection Matrix [1][1] = 1 / tan(fov/2)
    // We need tan(fov/2).
    let tan_half_fov = 1.0 / proj.m[1][1];
    let aspect = proj.m[1][1] / proj.m[0][0];

    // Light direction (fixed for now)
    let light_dir = Vec3::new(0.5, 1.0, 0.5).normalize();

    // Ray Marching Loop
    for y in 0..height {
        // Map y to [-1, 1] (NDC Y is Up)
        // Screen Y increases downwards (0 at top).
        // NDC Y: +1 at top, -1 at bottom.
        let ndc_y = 1.0 - 2.0 * (y as f32 + 0.5) / height as f32;
        let screen_y = ndc_y * tan_half_fov;

        for x in 0..width {
            let ndc_x = (2.0 * (x as f32 + 0.5) / width as f32 - 1.0) * aspect;
            let screen_x = ndc_x * tan_half_fov;

            // Ray Direction in Camera Space: (screen_x, screen_y, -1.0)
            // Transform to World Space
            // ray_dir = (right * screen_x + up * screen_y + forward).normalize()
            // (assuming 'forward' is -Z in camera space, so forward * 1.0 is correct if forward is normalized view direction)
            let ray_dir = (right * screen_x + up * screen_y + forward).normalize();

            let mut t = 0.1; // Near plane offset
            let max_dist = 100.0;
            let mut hit = false;
            let mut hit_color = 0;

            // Sphere Tracing
            for _ in 0..64 {
                let p = camera_pos + ray_dir * t;
                let (dist, color) = scene.map(p);

                if dist < 0.001 {
                    hit = true;
                    hit_color = color;
                    break;
                }

                t += dist;
                if t > max_dist {
                    break;
                }
            }

            if hit {
                let hit_pos = camera_pos + ray_dir * t;

                // Project hit position to Clip Space to get Depth
                let (view_pos, _) = view.transform_point(hit_pos);
                let (clip_pos, clip_w) = proj.transform_point(view_pos);

                // NDC Depth (-1 to 1 usually, or 0 to 1 depending on setup)
                // In this engine, zbuffer seems to store `z / w`.
                let depth = clip_pos.z / clip_w;

                // Check against existing Z-Buffer
                if zb.test_and_set(x as i32, y as i32, depth) {
                    // Lighting
                    let normal = scene.normal(hit_pos);
                    let diff = normal.dot(light_dir).max(0.0);
                    let ambient = 0.2;
                    let intensity = (diff + ambient).min(1.0);

                    // Apply lighting to color
                    let r = ((hit_color >> 16) & 0xFF) as f32 * intensity;
                    let g = ((hit_color >> 8) & 0xFF) as f32 * intensity;
                    let b = (hit_color & 0xFF) as f32 * intensity;

                    let final_color =
                        0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                    fb.set_pixel(x as i32, y as i32, final_color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_sdf() {
        let sphere = SdfObject {
            primitive: SdfPrimitive::Sphere {
                radius: 1.0,
                center: Vec3::new(0.0, 0.0, 0.0),
            },
            color: 0xFFFFFFFF,
        };

        // Point at (2,0,0) -> dist = 2 - 1 = 1
        let d = sphere.distance(Vec3::new(2.0, 0.0, 0.0));
        assert!((d - 1.0).abs() < 0.001);

        // Point at (0.5,0,0) -> dist = 0.5 - 1 = -0.5
        let d = sphere.distance(Vec3::new(0.5, 0.0, 0.0));
        assert!((d - (-0.5)).abs() < 0.001);
    }

    #[test]
    fn test_box_sdf() {
        let b = SdfObject {
            primitive: SdfPrimitive::Box {
                size: Vec3::new(1.0, 1.0, 1.0),
                center: Vec3::new(0.0, 0.0, 0.0),
            },
            color: 0xFFFFFFFF,
        };

        // Point at (2,0,0) -> dist = 2 - 1 = 1
        let d = b.distance(Vec3::new(2.0, 0.0, 0.0));
        assert!((d - 1.0).abs() < 0.001);

        // Point inside (0,0,0) -> dist = -1
        let d = b.distance(Vec3::new(0.0, 0.0, 0.0));
        assert!((d - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_scene_map() {
        let mut scene = SdfScene::new();
        scene.add(SdfObject {
            primitive: SdfPrimitive::Sphere {
                radius: 1.0,
                center: Vec3::new(0.0, 0.0, 0.0),
            },
            color: 0xFF0000FF, // Red
        });

        // Far away
        let (d, _) = scene.map(Vec3::new(10.0, 0.0, 0.0));
        assert!((d - 9.0).abs() < 0.001);
    }

    #[test]
    fn test_render_sdf_simple() {
        // Minimal render test
        let mut fb = Framebuffer::new(10, 10).unwrap();
        let mut zb = ZBuffer::new(10, 10).unwrap();
        let mut scene = SdfScene::new();
        scene.add(SdfObject {
            primitive: SdfPrimitive::Sphere {
                radius: 1.0,
                center: Vec3::new(0.0, 0.0, 0.0),
            },
            color: 0xFFFFFFFF,
        });

        // Camera at (0,0,5) looking at (0,0,0)
        let eye = Vec3::new(0.0, 0.0, 5.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);

        render_sdf(&mut fb, &mut zb, &scene, &view, &proj, eye);

        // Center pixel should hit sphere
        let p = fb.get_pixel(5, 5).unwrap();
        assert_ne!(p, 0xFF000000, "Center pixel should not be black");
    }
}
