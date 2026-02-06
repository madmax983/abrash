//! Camera abstraction for 3D graphics.

use crate::math::{Mat4, Vec3};

/// A 3D camera that manages view and projection matrices.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    /// Creates a new camera.
    pub fn new(
        position: Vec3,
        target: Vec3,
        up: Vec3,
        fov: f32,
        aspect: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            position,
            target,
            up,
            fov,
            aspect,
            near,
            far,
        }
    }

    /// Calculates the View matrix (World -> Camera space).
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.position, self.target, self.up)
    }

    /// Calculates the Projection matrix (Camera -> Clip space).
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective(self.fov, self.aspect, self.near, self.far)
    }

    /// Calculates the View-Projection matrix.
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
}
