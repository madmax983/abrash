//! Shape primitives for rendering.
//!
//! Provides polygon structures and generation functions.

use crate::math::{Vec2, Mat2};
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
                Vec2::new( half, -half),
                Vec2::new( half,  half),
                Vec2::new(-half,  half),
            ],
        }
    }

    /// Create a regular polygon with n sides and given radius
    pub fn regular(sides: usize, radius: f32) -> Self {
        let mut vertices = Vec::with_capacity(sides);
        for i in 0..sides {
            let angle = 2.0 * PI * (i as f32) / (sides as f32) - PI / 2.0;
            vertices.push(Vec2::new(
                radius * angle.cos(),
                radius * angle.sin(),
            ));
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
            vertices: self.vertices.iter()
                .map(|&v| matrix.transform(v))
                .collect(),
        }
    }

    /// Translate all vertices by an offset
    pub fn translate(&self, offset: Vec2) -> Self {
        Self {
            vertices: self.vertices.iter()
                .map(|&v| v + offset)
                .collect(),
        }
    }
}
