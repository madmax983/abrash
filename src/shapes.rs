//! Shape primitives for rendering.
//!
//! Provides polygon structures and generation functions.

use crate::math::{Mat2, Vec2};
use std::f32::consts::PI;

/// A polygon defined by its vertices
#[derive(Debug, Clone)]
pub struct Polygon {
    vertices: Vec<Vec2>,
}

impl Polygon {
    /// Create a polygon from a list of vertices
    pub fn new(vertices: Vec<Vec2>) -> Self {
        Self { vertices }
    }

    /// Create a square centered at origin with given side length
    pub fn square(side: f32) -> Self {
        let half = side / 2.0;
        Self {
            vertices: vec![
                Vec2::new(-half, -half),
                Vec2::new(half, -half),
                Vec2::new(half, half),
                Vec2::new(-half, half),
            ],
        }
    }

    /// Create a regular polygon with n sides and given radius
    pub fn regular(sides: usize, radius: f32) -> Self {
        let mut vertices = Vec::with_capacity(sides);
        for i in 0..sides {
            let angle = 2.0 * PI * (i as f32) / (sides as f32) - PI / 2.0;
            vertices.push(Vec2::new(radius * angle.cos(), radius * angle.sin()));
        }
        Self { vertices }
    }

    /// Get the vertices of the polygon
    pub fn vertices(&self) -> &[Vec2] {
        &self.vertices
    }

    /// Transform all vertices by a matrix
    pub fn transform(&self, matrix: &Mat2) -> Self {
        Self {
            vertices: self.vertices.iter().map(|&v| matrix.transform(v)).collect(),
        }
    }

    /// Translate all vertices by an offset
    pub fn translate(&self, offset: Vec2) -> Self {
        Self {
            vertices: self.vertices.iter().map(|&v| v + offset).collect(),
        }
    }

    /// Transform vertices in place (avoids allocation)
    pub fn transform_in_place(&mut self, matrix: &Mat2) {
        for v in self.vertices.iter_mut() {
            *v = matrix.transform(*v);
        }
    }

    /// Translate vertices in place (avoids allocation)
    pub fn translate_in_place(&mut self, offset: Vec2) {
        for v in self.vertices.iter_mut() {
            *v = *v + offset;
        }
    }
}

/// A triangle defined by three vertices
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub v0: Vec2,
    pub v1: Vec2,
    pub v2: Vec2,
}

impl Triangle {
    /// Create a new triangle from three vertices
    pub fn new(v0: Vec2, v1: Vec2, v2: Vec2) -> Self {
        Self { v0, v1, v2 }
    }

    /// Transform the triangle by a matrix
    pub fn transform(&self, matrix: &Mat2) -> Self {
        Self {
            v0: matrix.transform(self.v0),
            v1: matrix.transform(self.v1),
            v2: matrix.transform(self.v2),
        }
    }

    /// Translate the triangle by an offset
    pub fn translate(&self, offset: Vec2) -> Self {
        Self {
            v0: self.v0 + offset,
            v1: self.v1 + offset,
            v2: self.v2 + offset,
        }
    }
}
