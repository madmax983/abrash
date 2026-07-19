//! Signed Distance Field (SDF) Renderer
//!
//! A hybrid renderer that ray-marches SDF primitives and composites them
//! into the rasterized depth buffer. This allows for mathematically perfect
//! shapes (Spheres, Boxes) to interact with polygon meshes.

#![allow(warnings)]

use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, Vec2, Vec3};
use crate::zbuffer::ZBuffer;

/// Supported SDF Primitives.
#[derive(Clone, Copy, Debug)]
pub enum SdfPrimitive {
    /// A perfect sphere defined by a radius.
    Sphere {
        /// The distance from the center to the surface.
        radius: f32,
        /// The 3D coordinate of the sphere's origin.
        center: Vec3,
    },
    /// An axis-aligned bounding box.
    Box {
        /// The half-extents (width, height, depth) of the box.
        size: Vec3,
        /// The 3D coordinate of the box's origin.
        center: Vec3,
    },
    /// A donut-like shape.
    Torus {
        /// The distance from the center of the hole to the center of the tube.
        major_radius: f32,
        /// The radius of the tube itself.
        minor_radius: f32,
        /// The 3D coordinate of the torus's origin.
        center: Vec3,
    },
    /// An infinite flat surface.
    Plane {
        /// The normalized directional vector pointing away from the surface.
        normal: Vec3,
        /// The offset from the origin along the normal vector.
        distance: f32,
    },
    /// A cylinder with hemispherical ends.
    Capsule {
        /// The coordinate of the first endpoint of the inner line segment.
        start: Vec3,
        /// The coordinate of the second endpoint of the inner line segment.
        end: Vec3,
        /// The thickness radius expanding outward from the line segment.
        radius: f32,
    },
}

/// An object in the SDF scene.
#[derive(Clone, Copy, Debug)]
pub struct SdfObject {
    /// The geometric shape defining the bounds of this object.
    pub primitive: SdfPrimitive,
    /// The ARGB color applied to the surface when rendered.
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

/// A scene containing SDF objects.
pub struct SdfScene {
    /// The collection of all shapes present in the scene.
    pub objects: Vec<SdfObject>,
}

impl Default for SdfScene {
    fn default() -> Self {
        Self::new()
    }
}

impl SdfScene {
    #[must_use]
    /// Initializes a new, empty SDF scene.
    pub const fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    /// Inserts a new signed distance field object into the scene graph.
    pub fn add(&mut self, object: SdfObject) {
        self.objects.push(object);
    }

    /// Find the minimum distance to the scene.
    /// Returns (distance, color).
    #[must_use]
    pub fn map(&self, p: Vec3) -> (f32, u32) {
        let mut min_dist = f32::MAX;
        let mut min_color = 0xFF00_0000;

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
    let right = Vec3::new(view.m[0][0], view.m[1][0], view.m[2][0]).fast_normalize();
    let up = Vec3::new(view.m[0][1], view.m[1][1], view.m[2][1]).fast_normalize();
    let forward = Vec3::new(-view.m[0][2], -view.m[1][2], -view.m[2][2]).fast_normalize();

    // Calculate FoV factor
    // Projection Matrix [0][0] = 1 / (aspect * tan(fov/2))
    // Projection Matrix [1][1] = 1 / tan(fov/2)
    // We need tan(fov/2).
    let tan_half_fov = 1.0 / proj.m[1][1];
    let aspect = proj.m[1][1] / proj.m[0][0];

    // Light direction (fixed for now)
    let light_dir = Vec3::new(0.5, 1.0, 0.5).fast_normalize();

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
            // ray_dir = (right * screen_x + up * screen_y + forward).fast_normalize()
            // (assuming 'forward' is -Z in camera space, so forward * 1.0 is correct if forward is normalized view direction)
            let ray_dir = (right * screen_x + up * screen_y + forward).fast_normalize();

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
                        0xFF00_0000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
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
            color: 0xFFFF_FFFF,
        };

        struct TestCase {
            p: Vec3,
            expected: f32,
        }

        let cases = vec![
            TestCase {
                p: Vec3::new(2.0, 0.0, 0.0),
                expected: 1.0,
            },
            TestCase {
                p: Vec3::new(0.5, 0.0, 0.0),
                expected: -0.5,
            },
            TestCase {
                p: Vec3::new(0.0, 0.0, 0.0),
                expected: -1.0,
            },
            TestCase {
                p: Vec3::new(1.0, 0.0, 0.0),
                expected: 0.0,
            },
        ];

        for case in cases {
            let d = sphere.distance(case.p);
            assert!((d - case.expected).abs() < 0.001, "Failed for {:?}", case.p);
        }
    }

    #[test]
    fn test_box_sdf() {
        let b = SdfObject {
            primitive: SdfPrimitive::Box {
                size: Vec3::new(1.0, 1.0, 1.0),
                center: Vec3::new(0.0, 0.0, 0.0),
            },
            color: 0xFFFF_FFFF,
        };

        struct TestCase {
            p: Vec3,
            expected: f32,
        }

        let cases = vec![
            TestCase {
                p: Vec3::new(2.0, 0.0, 0.0),
                expected: 1.0,
            },
            TestCase {
                p: Vec3::new(0.0, 0.0, 0.0),
                expected: -1.0,
            },
            TestCase {
                p: Vec3::new(1.0, 0.0, 0.0),
                expected: 0.0,
            },
            TestCase {
                p: Vec3::new(0.5, 0.5, 0.5),
                expected: -0.5,
            },
        ];

        for case in cases {
            let d = b.distance(case.p);
            assert!((d - case.expected).abs() < 0.001, "Failed for {:?}", case.p);
        }
    }

    #[test]
    fn test_torus_sdf() {
        let torus = SdfObject {
            primitive: SdfPrimitive::Torus {
                major_radius: 2.0,
                minor_radius: 0.5,
                center: Vec3::new(0.0, 0.0, 0.0),
            },
            color: 0xFFFF_FFFF,
        };

        struct TestCase {
            p: Vec3,
            expected: f32,
        }

        let cases = vec![
            // Point exactly on the center of the torus tube
            TestCase {
                p: Vec3::new(2.0, 0.0, 0.0),
                expected: -0.5,
            },
            // Point on the surface of the torus
            TestCase {
                p: Vec3::new(2.5, 0.0, 0.0),
                expected: 0.0,
            },
            // Point inside the hole of the torus
            TestCase {
                p: Vec3::new(0.0, 0.0, 0.0),
                expected: 1.5,
            },
            // Point above the torus
            TestCase {
                p: Vec3::new(2.0, 1.0, 0.0),
                expected: 0.5,
            },
        ];

        for case in cases {
            let d = torus.distance(case.p);
            assert!((d - case.expected).abs() < 0.001, "Failed for {:?}", case.p);
        }
    }

    #[test]
    fn test_plane_sdf() {
        let plane = SdfObject {
            primitive: SdfPrimitive::Plane {
                normal: Vec3::new(0.0, 1.0, 0.0),
                distance: 1.0,
            },
            color: 0xFFFF_FFFF,
        };

        struct TestCase {
            p: Vec3,
            expected: f32,
        }

        let cases = vec![
            // Point above plane
            TestCase {
                p: Vec3::new(0.0, 2.0, 0.0),
                expected: 3.0,
            },
            // Point on plane
            TestCase {
                p: Vec3::new(0.0, -1.0, 0.0),
                expected: 0.0,
            },
            // Point below plane
            TestCase {
                p: Vec3::new(0.0, -2.0, 0.0),
                expected: -1.0,
            },
        ];

        for case in cases {
            let d = plane.distance(case.p);
            assert!((d - case.expected).abs() < 0.001, "Failed for {:?}", case.p);
        }
    }

    #[test]
    fn test_capsule_sdf() {
        let capsule = SdfObject {
            primitive: SdfPrimitive::Capsule {
                start: Vec3::new(0.0, -1.0, 0.0),
                end: Vec3::new(0.0, 1.0, 0.0),
                radius: 0.5,
            },
            color: 0xFFFF_FFFF,
        };

        struct TestCase {
            p: Vec3,
            expected: f32,
        }

        let cases = vec![
            // Point on the side, outside
            TestCase {
                p: Vec3::new(1.0, 0.0, 0.0),
                expected: 0.5,
            },
            // Point on the side, inside
            TestCase {
                p: Vec3::new(0.0, 0.0, 0.0),
                expected: -0.5,
            },
            // Point at the top cap, outside
            TestCase {
                p: Vec3::new(0.0, 2.0, 0.0),
                expected: 0.5,
            },
            // Point at the top cap, on surface
            TestCase {
                p: Vec3::new(0.0, 1.5, 0.0),
                expected: 0.0,
            },
            // Point beyond bottom cap
            TestCase {
                p: Vec3::new(0.0, -2.0, 0.0),
                expected: 0.5,
            },
        ];

        for case in cases {
            let d = capsule.distance(case.p);
            assert!((d - case.expected).abs() < 0.001, "Failed for {:?}", case.p);
        }
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
    fn test_scene_normal() {
        let mut scene = SdfScene::new();
        scene.add(SdfObject {
            primitive: SdfPrimitive::Sphere {
                radius: 1.0,
                center: Vec3::new(0.0, 0.0, 0.0),
            },
            color: 0xFFFF_FFFF,
        });

        // Normal on top
        let n = scene.normal(Vec3::new(0.0, 1.0, 0.0));
        assert!((n.x - 0.0).abs() < 0.01);
        assert!((n.y - 1.0).abs() < 0.01);
        assert!((n.z - 0.0).abs() < 0.01);

        // Normal on right side
        let n = scene.normal(Vec3::new(1.0, 0.0, 0.0));
        assert!((n.x - 1.0).abs() < 0.01);
        assert!((n.y - 0.0).abs() < 0.01);
        assert!((n.z - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_render_sdf_simple() -> Result<(), &'static str> {
        // Minimal render test
        let mut fb = Framebuffer::new(10, 10)?;
        let mut zb = ZBuffer::new(10, 10)?;
        let mut scene = SdfScene::new();
        scene.add(SdfObject {
            primitive: SdfPrimitive::Sphere {
                radius: 1.0,
                center: Vec3::new(0.0, 0.0, 0.0),
            },
            color: 0xFFFF_FFFF,
        });

        // Camera at (0,0,5) looking at (0,0,0)
        let eye = Vec3::new(0.0, 0.0, 5.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);

        render_sdf(&mut fb, &mut zb, &scene, &view, &proj, eye);

        // Center pixel should hit sphere
        let p = fb.get_pixel(5, 5).unwrap_or(0);
        assert_ne!(p, 0xFF00_0000, "Center pixel should not be black");

        Ok(())
    }
}
