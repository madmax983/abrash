//! Software Raytracer Module.
//!
//! Provides a CPU-based raytracer that supports triangles, spheres, and AABBs.
//! Features:
//! *   Recursive reflections
//! *   Phong shading
//! *   Hard shadows
//! *   BVH (AABB) acceleration

use crate::framebuffer::Framebuffer;
use crate::math::{Vec2, Vec3};
use crate::mesh::AABB;
use crate::scene::{Scene, SceneObject};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// A ray in 3D space, defined by an origin and a direction.
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
    pub inv_direction: Vec3, // Pre-computed for AABB intersection
}

impl Ray {
    /// Creates a new ray.
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
    #[must_use]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }

    /// Intersects the ray with a triangle defined by vertices v0, v1, v2.
    /// Returns `Some(Hit)` if intersection occurs, `None` otherwise.
    /// Uses Möller–Trumbore algorithm.
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
        // Correct normal orientation (should face the ray)
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

    /// Intersects the ray with an Axis-Aligned Bounding Box.
    /// Returns `true` if intersection occurs.
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
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub color: u32,
}

/// A simple raytracer.
pub struct RayTracer {
    pub max_bounces: u32,
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

/// A structure to hold pre-calculated world data for an object.
struct RenderObject<'a> {
    obj: &'a SceneObject,
    world_aabb: AABB,
}

impl RayTracer {
    /// Creates a new RayTracer with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders the scene to the framebuffer.
    pub fn render(&self, scene: &Scene, fb: &mut Framebuffer) {
        let width = fb.width();
        let height = fb.height();
        let aspect = width as f32 / height as f32;

        // Pre-calculate World AABBs
        let render_objects: Vec<RenderObject> = scene
            .objects
            .iter()
            .map(|obj| RenderObject {
                obj,
                world_aabb: obj.calculate_world_aabb(),
            })
            .collect();

        // Reconstruct Camera
        let view = scene.camera.view;
        let proj = scene.camera.proj;

        let cam_right = Vec3::new(view.m[0][0], view.m[0][1], view.m[0][2]);
        let cam_up = Vec3::new(view.m[1][0], view.m[1][1], view.m[1][2]);
        let cam_back = Vec3::new(view.m[2][0], view.m[2][1], view.m[2][2]);
        let cam_forward = cam_back * -1.0;

        // Extract Eye position: Eye = -R^T * T
        let tx = view.m[0][3];
        let ty = view.m[1][3];
        let tz = view.m[2][3];

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
        let iter = buffer.par_chunks_mut(width as usize).enumerate();
        #[cfg(not(feature = "parallel"))]
        let iter = buffer.chunks_mut(width as usize).enumerate();

        iter.for_each(|(y, row)| {
            let ndc_y = start_y - (y as f32 + 0.5) * pixel_height;
            for (x, pixel) in row.iter_mut().enumerate() {
                let ndc_x = start_x + (x as f32 + 0.5) * pixel_width;

                // Ray Direction
                let direction = (cam_forward + cam_right * ndc_x + cam_up * ndc_y).normalize();
                let ray = Ray::new(eye, direction);

                *pixel = self.trace_ray(&ray, &render_objects, 0);
            }
        });
    }

    fn trace_ray(&self, ray: &Ray, objects: &[RenderObject], depth: u32) -> u32 {
        if depth > self.max_bounces {
            return self.background_color;
        }

        let mut closest_hit: Option<Hit> = None;
        let mut closest_t = f32::MAX;
        let mut hit_obj: Option<&SceneObject> = None;

        for r_obj in objects {
            if !ray.intersect_aabb(&r_obj.world_aabb, 0.001, closest_t) {
                continue;
            }

            let mesh = &r_obj.obj.mesh;
            for indices in &mesh.indices {
                // Transform vertices to World Space
                let v0_local = mesh.vertices[indices[0]];
                let v1_local = mesh.vertices[indices[1]];
                let v2_local = mesh.vertices[indices[2]];

                let (v0, _) = r_obj.obj.transform.transform_point(v0_local);
                let (v1, _) = r_obj.obj.transform.transform_point(v1_local);
                let (v2, _) = r_obj.obj.transform.transform_point(v2_local);

                if let Some(hit) = ray.intersect_triangle(v0, v1, v2, 0.001, closest_t) {
                    if hit.t < closest_t {
                        closest_t = hit.t;
                        closest_hit = Some(hit);
                        hit_obj = Some(r_obj.obj);
                    }
                }
            }
        }

        if let Some(hit) = closest_hit {
            // Lighting
            let light_dir = Vec3::new(-0.5, -1.0, -0.3).normalize();
            let light_color = Vec3::new(1.0, 1.0, 1.0);
            let ambient = Vec3::new(0.1, 0.1, 0.1);

            let obj = hit_obj.unwrap();
            let base_color = obj.color;

            let r = ((base_color >> 16) & 0xFF) as f32 / 255.0;
            let g = ((base_color >> 8) & 0xFF) as f32 / 255.0;
            let b = (base_color & 0xFF) as f32 / 255.0;
            let material_color = Vec3::new(r, g, b);

            // Diffuse
            let diff = hit.normal.dot(light_dir * -1.0).max(0.0);
            let diffuse = light_color * diff;

            // Specular
            let view_dir = ray.direction * -1.0;
            let reflect_dir = reflect(light_dir, hit.normal).normalize();
            let spec = reflect_dir.dot(view_dir).max(0.0).powf(32.0);
            let specular = light_color * spec * 0.5;

            // Shadow
            let shadow_ray = Ray::new(hit.point + hit.normal * 0.001, light_dir * -1.0);
            let in_shadow = self.check_shadow(&shadow_ray, objects);
            let shadow_factor = if in_shadow { 0.2 } else { 1.0 };

            let final_color = (ambient + (diffuse + specular) * shadow_factor) * material_color;

            // Reflection (Simple 30% mix)
            let reflectivity = 0.3;
            let reflected_color = if depth < self.max_bounces {
                let r_ray = Ray::new(
                    hit.point + hit.normal * 0.001,
                    reflect(ray.direction, hit.normal),
                );
                let r_col_u32 = self.trace_ray(&r_ray, objects, depth + 1);
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

    fn check_shadow(&self, ray: &Ray, objects: &[RenderObject]) -> bool {
        for r_obj in objects {
            if !ray.intersect_aabb(&r_obj.world_aabb, 0.001, 1000.0) {
                continue;
            }
            let mesh = &r_obj.obj.mesh;
            for indices in &mesh.indices {
                let v0_local = mesh.vertices[indices[0]];
                let v1_local = mesh.vertices[indices[1]];
                let v2_local = mesh.vertices[indices[2]];

                let (v0, _) = r_obj.obj.transform.transform_point(v0_local);
                let (v1, _) = r_obj.obj.transform.transform_point(v1_local);
                let (v2, _) = r_obj.obj.transform.transform_point(v2_local);

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
