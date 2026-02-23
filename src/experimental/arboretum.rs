//! Arboretum: Procedural 3D Plant Generator using L-Systems.
//!
//! This module implements a Lindenmayer System interpreter that generates 3D meshes.
//! It uses a "Turtle Graphics" approach where a cursor moves through 3D space,
//! drawing geometry (branches) based on a string of commands.
//!
//! # Supported Symbols
//!
//! *   `F`: Move forward and draw a segment (cylinder).
//! *   `f`: Move forward without drawing (gap).
//! *   `+`: Turn Left (Yaw +).
//! *   `-`: Turn Right (Yaw -).
//! *   `&`: Pitch Down.
//! *   `^`: Pitch Up.
//! *   `\`: Roll Left.
//! *   `/`: Roll Right.
//! *   `|`: Turn 180 degrees.
//! *   `[`: Push state to stack.
//! *   `]`: Pop state from stack.

#![allow(warnings)]

use crate::math::Vec3;
use crate::mesh::Mesh;
use std::collections::HashMap;
use std::f32::consts::PI;

const MAX_ITERATIONS: u32 = 64;
const MAX_EXPANSION_LENGTH: usize = 64 * 1024 * 1024; // 64 MB limit

/// Represents the state of the drawing turtle.
#[derive(Clone, Debug)]
pub struct Turtle {
    /// Current position in 3D space.
    pub position: Vec3,
    /// Forward direction vector (normalized).
    pub heading: Vec3,
    /// Up direction vector (normalized).
    pub up: Vec3,
    /// Left direction vector (normalized).
    pub left: Vec3,
    /// Current step length (segment length).
    pub step_length: f32,
    /// Current width (radius) of the segment.
    pub radius: f32,
}

impl Turtle {
    /// Creates a new turtle at the origin, facing Up (Y+).
    pub fn new(step_length: f32, radius: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            heading: Vec3::new(0.0, 1.0, 0.0), // Grow up
            up: Vec3::new(0.0, 0.0, 1.0),      // Z is "depth" / up relative to heading
            left: Vec3::new(1.0, 0.0, 0.0),    // X is "width"
            step_length,
            radius,
        }
    }
}

/// Rotates vector `v` around axis `k` by `theta` radians using Rodrigues' rotation formula.
/// k must be a unit vector.
fn rotate_vector(v: Vec3, k: Vec3, theta: f32) -> Vec3 {
    let cos_theta = theta.cos();
    let sin_theta = theta.sin();

    // Term 1: v * cos(theta)
    let t1 = v * cos_theta;

    // Term 2: (k x v) * sin(theta)
    let t2 = k.cross(v) * sin_theta;

    // Term 3: k * (k . v) * (1 - cos(theta))
    let dot = k.dot(v);
    let t3 = k * dot * (1.0 - cos_theta);

    t1 + t2 + t3
}

/// A Lindenmayer System for generating plant structures.
pub struct LSystem {
    /// The initial string.
    pub axiom: String,
    /// Production rules: Key -> Replacement String.
    pub rules: HashMap<char, String>,
    /// Angle increment for rotations (in radians).
    pub angle: f32,
    /// Step length for movement.
    pub step_length: f32,
    /// Initial radius for segments.
    pub radius: f32,
}

impl LSystem {
    /// Creates a new L-System.
    pub fn new(axiom: &str, angle_degrees: f32, step_length: f32, radius: f32) -> Self {
        Self {
            axiom: axiom.to_string(),
            rules: HashMap::new(),
            angle: angle_degrees.to_radians(),
            step_length,
            radius,
        }
    }

    /// Adds a production rule.
    pub fn add_rule(&mut self, input: char, output: &str) {
        self.rules.insert(input, output.to_string());
    }

    /// Expands the axiom string by `iterations`.
    pub fn expand(&self, iterations: u32) -> Result<String, String> {
        if iterations > MAX_ITERATIONS {
            return Err(format!("Iterations {} exceeds limit {}", iterations, MAX_ITERATIONS));
        }

        let mut current = self.axiom.clone();

        for _ in 0..iterations {
            if current.len() > MAX_EXPANSION_LENGTH {
                return Err(format!("Expansion length {} exceeds limit {}", current.len(), MAX_EXPANSION_LENGTH));
            }

            // Estimate capacity: slightly more than current length (heuristic)
            // We check length during expansion to prevent massive pre-allocation if estimate is wrong.
            // Using saturating_mul to avoid overflow in capacity calculation (though Vec handles limits)
            let capacity = current.len().saturating_mul(2);
            // Don't allocate more than limit
            let capacity = capacity.min(MAX_EXPANSION_LENGTH + 1024);

            let mut next = String::with_capacity(capacity);
            for c in current.chars() {
                if let Some(replacement) = self.rules.get(&c) {
                    next.push_str(replacement);
                } else {
                    next.push(c);
                }
                if next.len() > MAX_EXPANSION_LENGTH {
                    return Err(format!("Expansion length exceeded limit {}", MAX_EXPANSION_LENGTH));
                }
            }
            current = next;
        }

        Ok(current)
    }

    /// Generates a Mesh from the expanded L-System string.
    pub fn generate_mesh(&self, iterations: u32) -> Result<Mesh, String> {
        let instructions = self.expand(iterations)?;
        let mut mesh = Mesh::new();
        let mut stack: Vec<Turtle> = Vec::new();
        let mut turtle = Turtle::new(self.step_length, self.radius);

        for c in instructions.chars() {
            match c {
                'F' => {
                    // Draw segment
                    let start = turtle.position;
                    let end = start + turtle.heading * turtle.step_length;

                    self.add_segment(&mut mesh, &turtle, start, end);

                    turtle.position = end;
                }
                'f' => {
                    // Move without drawing
                    turtle.position = turtle.position + turtle.heading * turtle.step_length;
                }
                '+' => {
                    // Yaw Left (around Up)
                    turtle.heading =
                        rotate_vector(turtle.heading, turtle.up, self.angle).normalize();
                    turtle.left = turtle.up.cross(turtle.heading).normalize();
                }
                '-' => {
                    // Yaw Right (around Up)
                    turtle.heading =
                        rotate_vector(turtle.heading, turtle.up, -self.angle).normalize();
                    turtle.left = turtle.up.cross(turtle.heading).normalize();
                }
                '&' => {
                    // Pitch Down (around Left)
                    turtle.heading =
                        rotate_vector(turtle.heading, turtle.left, self.angle).normalize();
                    turtle.up = turtle.heading.cross(turtle.left).normalize();
                }
                '^' => {
                    // Pitch Up (around Left)
                    turtle.heading =
                        rotate_vector(turtle.heading, turtle.left, -self.angle).normalize();
                    turtle.up = turtle.heading.cross(turtle.left).normalize();
                }
                '\\' => {
                    // Roll Left (around Heading)
                    turtle.up = rotate_vector(turtle.up, turtle.heading, self.angle).normalize();
                    turtle.left = turtle.up.cross(turtle.heading).normalize();
                }
                '/' => {
                    // Roll Right (around Heading)
                    turtle.up = rotate_vector(turtle.up, turtle.heading, -self.angle).normalize();
                    turtle.left = turtle.up.cross(turtle.heading).normalize();
                }
                '|' => {
                    // Turn 180 (around Up)
                    turtle.heading = rotate_vector(turtle.heading, turtle.up, PI).normalize();
                    turtle.left = turtle.up.cross(turtle.heading).normalize();
                }
                '[' => {
                    stack.push(turtle.clone());
                    // Stack depth limit could be added here too
                }
                ']' => {
                    if let Some(state) = stack.pop() {
                        turtle = state;
                    }
                }
                _ => {} // Ignore unknown symbols
            }
        }

        Ok(mesh)
    }

    /// Adds a 4-sided prism segment to the mesh.
    fn add_segment(&self, mesh: &mut Mesh, turtle: &Turtle, start: Vec3, end: Vec3) {
        // Calculate corner offsets based on turtle's Up and Left vectors
        // We use a square cross-section aligned with the turtle's frame
        let r = turtle.radius;

        // 4 corners in local space (relative to position)
        // c0: +Left, +Up
        // c1: -Left, +Up
        // c2: -Left, -Up
        // c3: +Left, -Up
        let corners = [
            turtle.left * r + turtle.up * r,
            turtle.left * -r + turtle.up * r,
            turtle.left * -r + turtle.up * -r,
            turtle.left * r + turtle.up * -r,
        ];

        // Start vertices
        let base_idx = mesh.vertices.len();
        for c in &corners {
            mesh.vertices.push(start + *c);
        }
        // End vertices
        for c in &corners {
            mesh.vertices.push(end + *c);
        }

        // Normals (approximate as face normals or vertex normals?)
        // For flat shading (prisms), we need duplicated vertices if we want sharp edges.
        // But here we are sharing vertices for the corners.
        // If we share vertices, the normal will be interpolated, making it look round (smooth shading).
        // Since `Mesh` doesn't enforce smooth/flat, let's just push normals pointing out from center.

        // Actually, let's just push vertices. Normals are optional for now or calculated later.
        // But `Mesh` expects `normals` if we want lighting.
        // Let's add normals pointing away from the segment axis.
        for c in &corners {
            mesh.normals.push(c.normalize());
        }
        for c in &corners {
            mesh.normals.push(c.normalize());
        }

        // Indices (Triangles)
        // 4 faces, 2 triangles each.
        // Vertices: 0-3 (start), 4-7 (end)
        // Face 0: 0, 4, 5, 1 (Side +Up) -> No, +Left/+Up is corner.
        // Let's connect the sides.
        // Side 1: 0 -> 4 -> 5 -> 1
        mesh.indices
            .push([base_idx + 0, base_idx + 4, base_idx + 5]);
        mesh.indices
            .push([base_idx + 0, base_idx + 5, base_idx + 1]);

        // Side 2: 1 -> 5 -> 6 -> 2
        mesh.indices
            .push([base_idx + 1, base_idx + 5, base_idx + 6]);
        mesh.indices
            .push([base_idx + 1, base_idx + 6, base_idx + 2]);

        // Side 3: 2 -> 6 -> 7 -> 3
        mesh.indices
            .push([base_idx + 2, base_idx + 6, base_idx + 7]);
        mesh.indices
            .push([base_idx + 2, base_idx + 7, base_idx + 3]);

        // Side 4: 3 -> 7 -> 4 -> 0
        mesh.indices
            .push([base_idx + 3, base_idx + 7, base_idx + 4]);
        mesh.indices
            .push([base_idx + 3, base_idx + 4, base_idx + 0]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expansion() {
        let mut lsys = LSystem::new("A", 90.0, 1.0, 0.1);
        lsys.add_rule('A', "AB");
        lsys.add_rule('B', "A");

        // Iteration 0: A
        assert_eq!(lsys.expand(0).unwrap(), "A");
        // Iteration 1: AB
        assert_eq!(lsys.expand(1).unwrap(), "AB");
        // Iteration 2: ABA
        assert_eq!(lsys.expand(2).unwrap(), "ABA");
        // Iteration 3: ABAAB
        assert_eq!(lsys.expand(3).unwrap(), "ABAAB");
    }

    #[test]
    fn test_mesh_generation() {
        // Simple "stick"
        let lsys = LSystem::new("F", 90.0, 1.0, 0.1);
        let mesh = lsys.generate_mesh(1).unwrap();

        // Should have 8 vertices (4 start, 4 end)
        assert_eq!(mesh.vertices.len(), 8);
        // Should have 8 triangles (4 faces * 2)
        assert_eq!(mesh.indices.len(), 8);
    }

    #[test]
    fn test_rotation() {
        let up = Vec3::new(0.0, 1.0, 0.0);
        let right = Vec3::new(1.0, 0.0, 0.0);

        // Rotate Right around Up by 90 degrees -> Forward (0, 0, -1) in RH system?
        // Wait, RH system: X=Right, Y=Up, Z=Back. Forward is -Z.
        // X cross Y = Z.
        // Rotate X around Y by 90 -> -Z.
        let v = rotate_vector(right, up, PI / 2.0);

        // Tolerance
        assert!((v.x - 0.0).abs() < 1e-5);
        assert!((v.y - 0.0).abs() < 1e-5);
        assert!((v.z - -1.0).abs() < 1e-5);
    }
}
