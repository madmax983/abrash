//! Software Raytracer Module.
//!
//! Provides a simple CPU-based recursive raytracer (Whitted-style) that supports triangles and AABBs.
//!
//! # Purpose
//!
//! This module serves as a **reference implementation** for scene verification. It is *not* intended
//! for real-time rendering. It helps verify:
//!
//! 1.  **Coordinate Systems**: Ensuring that the Camera View matrix and World transformations are correct.
//! 2.  **Geometry**: Verifying that meshes are loaded and transformed correctly without rasterization artifacts.
//! 3.  **Lighting Reference**: Providing a ground truth for simple Phong shading to compare against rasterized shaders.
//!
//! # Features
//!
//! *   **Recursive Reflections**: Supports hard-coded reflection depth (default: 3 bounces).
//! *   **Phong Shading**: Implements Ambient + Diffuse + Specular lighting.
//! *   **Hard Shadows**: Single directional light source with ray-casted shadows.
//! *   **BVH Acceleration**: Uses a simple Object-level AABB check to skip expensive mesh intersections.
//! *   **Parallel Rendering**: Uses `rayon` (if enabled) to trace rays in parallel.
//!
//! # Usage
//!
//! ```no_run
//! use abrash::experimental::raytracer::RayTracer;
//! use abrash::scene::{Scene, Camera};
//! use abrash::math::{Mat4, Vec3};
//! use abrash::framebuffer::Framebuffer;
//!
//! // 1. Setup Scene
//! let view = Mat4::look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
//! let proj = Mat4::perspective(1.57, 1.33, 0.1, 100.0);
//! let camera = Camera::new(view, proj);
//! let scene = Scene::new(camera);
//! // ... add objects to scene ...
//!
//! // 2. Setup Framebuffer
//! let mut fb = Framebuffer::new(800, 600).unwrap();
//!
//! // 3. Render
//! let tracer = RayTracer::new();
//! tracer.render(&scene, &mut fb);
//! ```

use crate::framebuffer::Framebuffer;
use crate::geometry::AABB;
use crate::math::{Vec2, Vec3};
use crate::scene::{Scene, SceneObject};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// A ray in 3D space, defined by an origin and a direction.
///
/// Used for intersection tests against scene geometry.
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    /// Starting point of the ray.
    pub origin: Vec3,
    /// Normalized direction vector.
    pub direction: Vec3,
    /// Reciprocal of direction (1.0 / direction), pre-computed for fast AABB intersection.
    pub inv_direction: Vec3,
}

impl Ray {
    /// Creates a new ray.
    ///
    /// # Arguments
    ///
    /// * `origin` - The starting position of the ray.
    /// * `direction` - The direction vector (will be normalized).
    #[must_use]
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        let direction = direction.normalize();
        Self {
            origin,
            direction,
            inv_direction: Vec3::new(1.0 / direction.x, 1.0 / direction.y, 1.0 / direction.z),
        }
    }

    /// Returns the point at distance `t` along the ray.
    ///
    /// $$ P(t) = Origin + Direction \cdot t $$
    #[must_use]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// Intersects the ray with a triangle defined by vertices v0, v1, v2.
    ///
    /// Uses the [Möller–Trumbore intersection algorithm](https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm).
    ///
    /// # Arguments
    ///
    /// * `v0`, `v1`, `v2` - Vertices of the triangle in World Space.
    /// * `t_min`, `t_max` - Valid range for the intersection distance `t`.
    ///
    /// # Returns
    ///
    /// * `Some(Hit)` if the ray intersects the triangle within the range `[t_min, t_max]`.
    /// * `None` otherwise.
    #[must_use]
    pub fn intersect_triangle(
        &self,
        v0: Vec3,
        v1: Vec3,
        v2: Vec3,
        t_min: f32,
        t_max: f32,
    ) -> Option<Hit> {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let h = self.direction.cross(edge2);
        let a = edge1.dot(h);

        if a.abs() < 1e-6 {
            return None; // Ray is parallel to triangle
        }

        let f = 1.0 / a;
        let s = self.origin - v0;
        let u = f * s.dot(h);

        if !(0.0..=1.0).contains(&u) {
            return None;
        }

        let q = s.cross(edge1);
        let v = f * self.direction.dot(q);

        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = f * edge2.dot(q);

        if t < t_min || t > t_max {
            return None;
        }

        // Compute normal
        let normal = edge1.cross(edge2).normalize();
        // Correct normal orientation (double-sided lighting)
        // If the normal points away from the ray (dot > 0), flip it.
        let normal = if normal.dot(self.direction) > 0.0 {
            normal * -1.0
        } else {
            normal
        };

        Some(Hit {
            t,
            point: self.at(t),
            normal,
            uv: Vec2::new(u, v),
            color: 0xFFFFFFFF,
        })
    }

    /// Intersects the ray with an Axis-Aligned Bounding Box (AABB).
    ///
    /// Uses the "Slab Method" optimization.
    ///
    /// # Returns
    ///
    /// * `true` if the ray intersects the AABB within `[t_min, t_max]`.
    #[must_use]
    pub fn intersect_aabb(&self, aabb: &AABB, t_min: f32, t_max: f32) -> bool {
        let tx1 = (aabb.min.x - self.origin.x) * self.inv_direction.x;
        let tx2 = (aabb.max.x - self.origin.x) * self.inv_direction.x;

        let tmin = tx1.min(tx2);
        let tmax = tx1.max(tx2);

        let ty1 = (aabb.min.y - self.origin.y) * self.inv_direction.y;
        let ty2 = (aabb.max.y - self.origin.y) * self.inv_direction.y;

        let tmin = tmin.max(ty1.min(ty2));
        let tmax = tmax.min(ty1.max(ty2));

        let tz1 = (aabb.min.z - self.origin.z) * self.inv_direction.z;
        let tz2 = (aabb.max.z - self.origin.z) * self.inv_direction.z;

        let tmin = tmin.max(tz1.min(tz2));
        let tmax = tmax.min(tz1.max(tz2));

        tmax >= tmin && tmax > t_min && tmin < t_max
    }
}

/// Information about a ray-object intersection.
#[derive(Debug, Clone, Copy)]
pub struct Hit {
    /// Distance from ray origin to intersection point.
    pub t: f32,
    /// Intersection point in World Space.
    pub point: Vec3,
    /// Surface normal at the intersection point.
    pub normal: Vec3,
    /// Barycentric coordinates (u, v) of the hit.
    pub uv: Vec2,
    /// Interpolated color at the hit (if supported).
    pub color: u32,
}

/// A simple recursive raytracer.
///
/// See the [module-level documentation](self) for usage.
#[doc(alias = "PathTracer")]
pub struct RayTracer {
    /// Maximum number of reflection bounces (recursion depth).
    pub max_bounces: u32,
    /// Color returned when a ray hits nothing (ARGB).
    pub background_color: u32,
}

impl Default for RayTracer {
    fn default() -> Self {
        Self {
            max_bounces: 3,
            background_color: 0xFF101010, // Dark Grey
        }
    }
}

impl RayTracer {
    /// Creates a new `RayTracer` with default settings (3 bounces, dark grey background).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders the scene to the framebuffer using ray tracing.
    ///
    /// This method iterates over every pixel in the framebuffer, generating a primary ray
    /// from the camera eye through the pixel on the image plane.
    ///
    /// # Performance
    ///
    /// This implementation performs a brute-force intersection against all triangles
    /// in visible objects. It includes a basic optimization: objects are first checked
    /// against their World AABB before testing triangles.
    ///
    /// If the `parallel` feature is enabled, this method uses `rayon` to trace rays
    /// in parallel across multiple threads.
    /// Bolt Performance Optimization:
    /// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
    /// remainder chunk handling and bounds checking, enabling better vectorization
    /// and measurable performance improvements.
    /// Bolt Performance Optimization:
    /// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
    /// remainder chunk handling and bounds checking, enabling better vectorization
    /// and measurable performance improvements.
    /// Bolt Performance Optimization:
    /// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
    /// remainder chunk handling and bounds checking, enabling better vectorization
    /// and measurable performance improvements.
    /// Bolt Performance Optimization:
    /// Replaced `.chunks_mut(width)` with `.chunks_exact_mut(width)` to eliminate
    /// remainder chunk handling and bounds checking, enabling better vectorization
    /// and measurable performance improvements.
    pub fn render(&self, scene: &Scene, fb: &mut Framebuffer) {
        let width = fb.width();
        let height = fb.height();
        let aspect = width as f32 / height as f32;

        // Pre-calculate World AABBs
        thread_local! {
            static AABB_BUFFER: std::cell::RefCell<Vec<AABB>> = const { std::cell::RefCell::new(Vec::new()) };
        }

        // Reconstruct Camera Vectors from View Matrix.
        // View Matrix is R * T (Row-Major).
        // The rotation submatrix R transforms World basis to View basis.
        // The inverse R^T transforms View basis to World basis.
        // So the columns of R are the World-Space Right, Up, and Back vectors.
        let view = scene.camera.view;
        let proj = scene.camera.proj;

        let cam_right = Vec3::new(view.m[0][0], view.m[1][0], view.m[2][0]); // Column 0
        let cam_up = Vec3::new(view.m[0][1], view.m[1][1], view.m[2][1]); // Column 1
        let cam_back = Vec3::new(view.m[0][2], view.m[1][2], view.m[2][2]); // Column 2
        let cam_forward = cam_back * -1.0;

        // Extract Eye position.
        // The translation row T (row 3) contains the dot products of -eye with the basis vectors.
        // T = (-eye . Right, -eye . Up, -eye . Back)
        // So eye = -(T.x * Right + T.y * Up + T.z * Back)
        let tx = view.m[3][0];
        let ty = view.m[3][1];
        let tz = view.m[3][2];

        let eye = Vec3::new(
            -(cam_right.x * tx + cam_up.x * ty + cam_back.x * tz),
            -(cam_right.y * tx + cam_up.y * ty + cam_back.y * tz),
            -(cam_right.z * tx + cam_up.z * ty + cam_back.z * tz),
        );

        // Focal length and plane dimensions
        let plane_height = 2.0 / proj.m[1][1];
        let plane_width = plane_height * aspect;

        let pixel_width = plane_width / width as f32;
        let pixel_height = plane_height / height as f32;

        let start_x = -plane_width * 0.5;
        let start_y = plane_height * 0.5;

        // Parallel Loop
        let buffer = fb.as_mut_slice();

        #[cfg(feature = "parallel")]
        let iter = buffer.par_chunks_exact_mut(width as usize).enumerate();
        #[cfg(not(feature = "parallel"))]
        let iter = buffer.chunks_exact_mut(width as usize).enumerate();

        AABB_BUFFER.with(|buffer| {
            let mut world_aabbs = buffer.borrow_mut();
            world_aabbs.clear();
            world_aabbs.reserve(scene.objects.len());
            world_aabbs.extend(
                scene.objects
                    .iter()
                    .map(|obj| obj.calculate_world_aabb()),
            );
        });

        AABB_BUFFER.with(|buffer| {
            let world_aabbs = buffer.borrow();

            // Extract immutable slices to satisfy the parallel iterator
            let objects_slice: &[SceneObject] = &scene.objects;
            let aabbs_slice: &[AABB] = &world_aabbs;

            iter.for_each(|(y, row)| {
                let ndc_y = start_y - (y as f32 + 0.5) * pixel_height;
                for (x, pixel) in row.iter_mut().enumerate() {
                    let ndc_x = start_x + (x as f32 + 0.5) * pixel_width;

                    // Ray Direction
                    let direction = (cam_forward + cam_right * ndc_x + cam_up * ndc_y).normalize();
                    let ray = Ray::new(eye, direction);

                    *pixel = self.trace_ray(&ray, objects_slice, aabbs_slice, 0);
                }
            });
        });
    }

    fn trace_ray(&self, ray: &Ray, objects: &[SceneObject], aabbs: &[AABB], depth: u32) -> u32 {
        if depth > self.max_bounces {
            return self.background_color;
        }

        let mut closest_hit: Option<(Hit, &SceneObject)> = None;
        let mut closest_t = f32::MAX;

        for (obj, world_aabb) in objects.iter().zip(aabbs.iter()) {
            if !ray.intersect_aabb(world_aabb, 0.001, closest_t) {
                continue;
            }

            let mesh = &obj.mesh;
            for indices in &mesh.indices {
                // Transform vertices to World Space
                let v0_local = mesh.vertices[indices[0]];
                let v1_local = mesh.vertices[indices[1]];
                let v2_local = mesh.vertices[indices[2]];

                let (v0, _) = obj.transform.transform_point(v0_local);
                let (v1, _) = obj.transform.transform_point(v1_local);
                let (v2, _) = obj.transform.transform_point(v2_local);

                if let Some(hit) = ray.intersect_triangle(v0, v1, v2, 0.001, closest_t) {
                    closest_t = hit.t;
                    closest_hit = Some((hit, obj));
                }
            }
        }

        if let Some((hit, obj)) = closest_hit {
            // Lighting
            // Light source: Directional light from top-left-front
            let light_dir = Vec3::new(-0.5, -1.0, -0.3).normalize();
            let light_color = Vec3::new(1.0, 1.0, 1.0);
            let ambient = Vec3::new(0.1, 0.1, 0.1);

            let base_color = obj.color;

            let r = ((base_color >> 16) & 0xFF) as f32 / 255.0;
            let g = ((base_color >> 8) & 0xFF) as f32 / 255.0;
            let b = (base_color & 0xFF) as f32 / 255.0;
            let material_color = Vec3::new(r, g, b);

            // Diffuse (Lambert)
            let diff = hit.normal.dot(light_dir * -1.0).max(0.0);
            let diffuse = light_color * diff;

            // Specular (Phong)
            let view_dir = ray.direction * -1.0;
            let reflect_dir = reflect(light_dir, hit.normal).normalize();
            let spec = reflect_dir.dot(view_dir).max(0.0).powf(32.0);
            let specular = light_color * spec * 0.5;

            // Shadow Ray
            let shadow_ray = Ray::new(hit.point + hit.normal * 0.001, light_dir * -1.0);
            let in_shadow = Self::check_shadow(&shadow_ray, objects, aabbs);
            let shadow_factor = if in_shadow { 0.2 } else { 1.0 };

            let final_color = (ambient + (diffuse + specular) * shadow_factor) * material_color;

            // Reflection (Recursive)
            // Mix 30% reflection with 70% base color
            let reflectivity = 0.3;
            let reflected_color = if depth < self.max_bounces {
                let r_ray = Ray::new(
                    hit.point + hit.normal * 0.001,
                    reflect(ray.direction, hit.normal),
                );
                let r_col_u32 = self.trace_ray(&r_ray, objects, aabbs, depth + 1);
                let rr = ((r_col_u32 >> 16) & 0xFF) as f32 / 255.0;
                let rg = ((r_col_u32 >> 8) & 0xFF) as f32 / 255.0;
                let rb = (r_col_u32 & 0xFF) as f32 / 255.0;
                Vec3::new(rr, rg, rb)
            } else {
                Vec3::default()
            };

            let mixed = final_color.lerp(reflected_color, reflectivity);
            let mixed = Vec3::new(mixed.x.min(1.0), mixed.y.min(1.0), mixed.z.min(1.0));

            return 0xFF000000
                | ((mixed.x * 255.0) as u32) << 16
                | ((mixed.y * 255.0) as u32) << 8
                | ((mixed.z * 255.0) as u32);
        }

        self.background_color
    }

    fn check_shadow(ray: &Ray, objects: &[SceneObject], aabbs: &[AABB]) -> bool {
        for (obj, world_aabb) in objects.iter().zip(aabbs.iter()) {
            if !ray.intersect_aabb(world_aabb, 0.001, 1000.0) {
                continue;
            }
            let mesh = &obj.mesh;
            for indices in &mesh.indices {
                let v0_local = mesh.vertices[indices[0]];
                let v1_local = mesh.vertices[indices[1]];
                let v2_local = mesh.vertices[indices[2]];

                let (v0, _) = obj.transform.transform_point(v0_local);
                let (v1, _) = obj.transform.transform_point(v1_local);
                let (v2, _) = obj.transform.transform_point(v2_local);

                if ray.intersect_triangle(v0, v1, v2, 0.001, 1000.0).is_some() {
                    return true;
                }
            }
        }
        false
    }
}

fn reflect(v: Vec3, n: Vec3) -> Vec3 {
    v - n * 2.0 * v.dot(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Mat4;
    use crate::mesh::Mesh;
    use crate::scene::Camera;

    #[test]
    fn should_return_background_color_on_miss() {
        let tracer = RayTracer {
            background_color: 0xFF123456,
            ..Default::default()
        };

        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
        let camera = Camera::new(view, proj);
        let scene = Scene::new(camera);

        let mut fb = Framebuffer::new(10, 10).unwrap();
        tracer.render(&scene, &mut fb);

        // Ray misses everything, so every pixel should be background_color
        for y in 0..10 {
            for x in 0..10 {
                assert_eq!(fb.get_pixel(x, y).unwrap(), 0xFF123456);
            }
        }
    }

    #[test]
    fn should_render_object_color_on_hit() {
        let tracer = RayTracer {
            background_color: 0xFF000000,
            ..Default::default()
        };

        let view = Mat4::look_at(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
        let camera = Camera::new(view, proj);
        let mut scene = Scene::new(camera);

        // A large quad that covers the screen
        let mut mesh = Mesh::new();
        mesh.vertices = vec![
            Vec3::new(-10.0, -10.0, 0.0),
            Vec3::new(10.0, -10.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
            Vec3::new(-10.0, 10.0, 0.0),
        ];
        mesh.indices = vec![[0, 1, 2], [0, 2, 3]];

        // Base color is pure red
        let red = 0xFFFF0000;
        let transform = Mat4::identity();
        scene.add_object(SceneObject::new(std::sync::Arc::new(mesh), transform, red));

        let mut fb = Framebuffer::new(10, 10).unwrap();
        tracer.render(&scene, &mut fb);

        // Due to lighting, the pixel at the center won't be exactly red,
        // but it should not be the background color. Let's check the middle pixel.
        let pixel = fb.get_pixel(5, 5).unwrap();
        assert_ne!(
            pixel, 0xFF000000,
            "Pixel should be shaded, not background color"
        );

        // Extract the red channel. Due to specular/ambient/diffuse, it should be > 0.
        let r = (pixel >> 16) & 0xFF;
        assert!(r > 0, "Red channel should be lit");
    }
}
