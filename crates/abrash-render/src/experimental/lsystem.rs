//! # Procedural L-System Generation
//!
//! An experimental module for generating procedural 3D meshes (like trees, ferns, and plants)
//! using Lindenmayer Systems (L-Systems).
//!
//! ## Overview
//!
//! L-Systems consist of an initial axiom string and a set of rewrite rules. Over multiple iterations,
//! the string is expanded. This string is then interpreted by a 3D "Turtle" to generate geometry.
//!
//! ### Turtle Commands
//! * `F`: Move forward and draw a segment
//! * `f`: Move forward without drawing
//! * `+`: Turn Right (Yaw)
//! * `-`: Turn Left (Yaw)
//! * `&`: Pitch Down
//! * `^`: Pitch Up
//! * `\`: Roll Left
//! * `/`: Roll Right
//! * `[`: Push State (Save Position & Orientation)
//! * `]`: Pop State (Restore Position & Orientation)

use crate::math::Vec3;
use crate::mesh::Mesh;
use std::collections::HashMap;

/// Configuration for an L-System generator.
#[derive(Debug, Clone)]
pub struct LSystem {
    axiom: String,
    rules: HashMap<char, String>,
    /// Maximum allowed string length to prevent OOM
    max_capacity: usize,
}

impl LSystem {
    /// Create a new L-System.
    #[must_use]
    pub fn new(axiom: &str) -> Self {
        Self {
            axiom: axiom.to_string(),
            rules: HashMap::new(),
            max_capacity: 100_000, // 100k char limit by default
        }
    }

    /// Add a replacement rule.
    pub fn add_rule(&mut self, predecessor: char, successor: &str) {
        self.rules.insert(predecessor, successor.to_string());
    }

    /// Set the maximum string expansion capacity to prevent memory exhaustion.
    pub const fn set_max_capacity(&mut self, capacity: usize) {
        self.max_capacity = capacity;
    }

    /// Expand the L-System string for the given number of iterations.
    ///
    /// # Errors
    /// Returns an error if the expansion string exceeds `max_capacity`.
    pub fn expand(&self, iterations: usize) -> Result<String, &'static str> {
        let mut current = self.axiom.clone();

        if iterations == 0 {
            return Ok(current);
        }

        let mut is_pure_ascii = self.axiom.is_ascii();
        if is_pure_ascii {
            for v in self.rules.values() {
                if !v.is_ascii() {
                    is_pure_ascii = false;
                    break;
                }
            }
            if is_pure_ascii {
                for k in self.rules.keys() {
                    if !k.is_ascii() {
                        is_pure_ascii = false;
                        break;
                    }
                }
            }
        }

        if is_pure_ascii {
            let mut rules_array: [Option<&[u8]>; 128] = [None; 128];
            for (k, v) in &self.rules {
                rules_array[(*k as usize) & 127] = Some(v.as_bytes());
            }

            let mut current_bytes = self.axiom.as_bytes().to_vec();
            let mut next_bytes = Vec::new();

            for _ in 0..iterations {
                next_bytes.clear();
                next_bytes.reserve(current_bytes.len() * 2);
                for &b in &current_bytes {
                    if let Some(replacement) = rules_array[(b as usize) & 127] {
                        next_bytes.extend_from_slice(replacement);
                    } else {
                        next_bytes.push(b);
                    }
                    if next_bytes.len() > self.max_capacity {
                        return Err("L-System expansion exceeded maximum capacity limit");
                    }
                }
                std::mem::swap(&mut current_bytes, &mut next_bytes);
            }

            // Safe because we already verified all rules and the axiom are pure ASCII.
            // Bolt Performance Optimization:
            // Reconstruct string from raw bytes directly avoiding unicode character parsing overhead
            return String::from_utf8(current_bytes).map_err(|_| "L-System utf8 decoding error");
        }

        let mut next_string = String::with_capacity(current.len() * 2);

        // Bolt Performance Optimization:
        // By pre-calculating a flat array for ASCII replacement lookups,
        // we bypass the `HashMap::get` and `SipHash` overhead entirely in the inner loop.
        let mut rules_array: [Option<&str>; 128] = [None; 128];
        for (k, v) in &self.rules {
            let u = *k as u32;
            if u < 128 {
                rules_array[u as usize] = Some(v.as_str());
            }
        }

        for _ in 0..iterations {
            next_string.clear();

            // Bolt Performance Optimization:
            // When `current` and `next_string` are swapped, the smaller buffer is recycled.
            // By reserving capacity before pushing new characters, we prevent continuous O(N)
            // heap reallocations as the string expands exponentially.
            next_string.reserve(current.len() * 2);

            for b in current.bytes() {
                let idx = b as usize;
                if idx < 128 {
                    if let Some(replacement) = rules_array[idx] {
                        next_string.push_str(replacement);
                    } else {
                        next_string.push(b as char);
                    }
                } else if let Some(replacement) = self.rules.get(&(b as char)) {
                    // Fallback for non-ASCII
                    next_string.push_str(replacement);
                } else {
                    next_string.push(b as char);
                }

                // OOM Prevention check
                if next_string.len() > self.max_capacity {
                    return Err("L-System expansion exceeded maximum capacity limit");
                }
            }

            std::mem::swap(&mut current, &mut next_string);
        }

        Ok(current)
    }
}

#[derive(Debug, Clone)]
struct TurtleState {
    position: Vec3,
    direction: Vec3,
    up: Vec3,
    right: Vec3,
    segment_length: f32,
    segment_radius: f32,
}

/// A stateful 3D Turtle for interpreting strings into meshes.
#[derive(Debug, Clone)]
pub struct Turtle {
    pub position: Vec3,
    pub direction: Vec3,
    pub up: Vec3,
    pub right: Vec3,
    pub segment_length: f32,
    pub segment_radius: f32,
    pub turn_angle: f32, // in radians
    stack: Vec<TurtleState>,
}

impl Turtle {
    /// Create a new turtle at the origin facing up (+Y).
    #[must_use]
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            direction: Vec3::new(0.0, 1.0, 0.0), // Default up
            up: Vec3::new(0.0, 0.0, -1.0),
            right: Vec3::new(1.0, 0.0, 0.0),
            segment_length: 1.0,
            segment_radius: 0.1,
            turn_angle: std::f32::consts::PI / 6.0, // 30 degrees
            stack: Vec::new(),
        }
    }

    /// Interpret an L-System string and generate a 3D Mesh.
    ///
    /// # Errors
    /// Returns an error if the stack overflows or underflows.
    pub fn generate_mesh(&mut self, commands: &str) -> Result<Mesh, &'static str> {
        // Base mesh to use for segments (a simple tetrahedron/pyramid or line placeholder)
        // For simplicity and speed in software rendering, we will generate a very simple geometry
        // per line segment (e.g., a small box or custom geometry). We will use a fast hardcoded method here.

        // Bolt Performance Optimization:
        // - Converts commands to an ASCII byte array to avoid UTF-8 `chars()` decoding overhead during iteration.
        // - Pre-calculates the required number of segments by scanning for `b'F'`.
        // - Uses `Mesh::with_capacity` to pre-allocate exact vertex and index buffers, preventing dynamic heap
        //   reallocations inside the hot interpretation loop. The `bytecount` crate was avoided to minimize dependencies.
        let commands_bytes = commands.as_bytes();
        #[allow(clippy::naive_bytecount)]
        let num_segments = commands_bytes.iter().filter(|&&b| b == b'F').count();
        let mut mesh = Mesh::with_capacity(num_segments * 8, num_segments * 12);

        for &c in commands_bytes {
            match c {
                b'F' => {
                    // Draw forward
                    let start = self.position;
                    self.position = self.position + self.direction * self.segment_length;
                    let end = self.position;
                    self.add_segment(&mut mesh, start, end, self.segment_radius);
                }
                b'f' => {
                    // Move forward (no draw)
                    self.position = self.position + self.direction * self.segment_length;
                }
                b'+' => self.yaw(self.turn_angle),
                b'-' => self.yaw(-self.turn_angle),
                b'&' => self.pitch(self.turn_angle),
                b'^' => self.pitch(-self.turn_angle),
                b'\\' => self.roll(self.turn_angle),
                b'/' => self.roll(-self.turn_angle),
                b'[' => {
                    if self.stack.len() > 1000 {
                        return Err("L-System stack overflow");
                    }
                    self.stack.push(TurtleState {
                        position: self.position,
                        direction: self.direction,
                        up: self.up,
                        right: self.right,
                        segment_length: self.segment_length,
                        segment_radius: self.segment_radius,
                    });
                }
                b']' => {
                    if let Some(state) = self.stack.pop() {
                        self.position = state.position;
                        self.direction = state.direction;
                        self.up = state.up;
                        self.right = state.right;
                        self.segment_length = state.segment_length;
                        self.segment_radius = state.segment_radius;
                    } else {
                        return Err("L-System stack underflow");
                    }
                }
                _ => {} // Ignore unknown symbols (like rule placeholders 'X', 'Y')
            }
        }

        // To ensure mesh generation was successful, make sure there are some normals
        let computed_normals = mesh.compute_face_normals();
        mesh.normals.clear();
        mesh.normals
            .resize(mesh.vertices.len(), Vec3::new(0.0, 1.0, 0.0));
        // Flat shading normals - assigning face normal to all vertices for simplicity
        for (i, face) in mesh.indices.iter().enumerate() {
            let n = computed_normals[i];
            mesh.normals[face[0]] = n;
            mesh.normals[face[1]] = n;
            mesh.normals[face[2]] = n;
        }

        Ok(mesh)
    }

    fn yaw(&mut self, angle: f32) {
        let (sin_a, cos_a) = angle.sin_cos();
        let new_dir = self.direction * cos_a + self.right * sin_a;
        let new_right = self.right * cos_a - self.direction * sin_a;
        self.direction = new_dir.normalize();
        self.right = new_right.normalize();
    }

    fn pitch(&mut self, angle: f32) {
        let (sin_a, cos_a) = angle.sin_cos();
        let new_dir = self.direction * cos_a + self.up * sin_a;
        let new_up = self.up * cos_a - self.direction * sin_a;
        self.direction = new_dir.normalize();
        self.up = new_up.normalize();
    }

    fn roll(&mut self, angle: f32) {
        let (sin_a, cos_a) = angle.sin_cos();
        let new_right = self.right * cos_a + self.up * sin_a;
        let new_up = self.up * cos_a - self.right * sin_a;
        self.right = new_right.normalize();
        self.up = new_up.normalize();
    }

    /// Adds a basic 3D rectangular prism (or diamond) segment representing a branch
    fn add_segment(&self, mesh: &mut Mesh, start: Vec3, end: Vec3, radius: f32) {
        let r = radius;
        // Generate a 4-sided branch segment (diamond-shaped cross-section)
        let right_offset = self.right * r;
        let up_offset = self.up * r;

        let base_index = mesh.vertices.len();

        // Bottom vertices
        mesh.vertices.push(start + right_offset); // 0
        mesh.vertices.push(start + up_offset); // 1
        mesh.vertices.push(start - right_offset); // 2
        mesh.vertices.push(start - up_offset); // 3

        // Top vertices
        let r_top = radius * 0.9; // Slight tapering
        let right_top = self.right * r_top;
        let up_top = self.up * r_top;

        mesh.vertices.push(end + right_top); // 4
        mesh.vertices.push(end + up_top); // 5
        mesh.vertices.push(end - right_top); // 6
        mesh.vertices.push(end - up_top); // 7

        // Bottom face
        mesh.indices
            .push([base_index, base_index + 2, base_index + 1]);
        mesh.indices
            .push([base_index, base_index + 3, base_index + 2]);

        // Top face
        mesh.indices
            .push([base_index + 4, base_index + 5, base_index + 6]);
        mesh.indices
            .push([base_index + 4, base_index + 6, base_index + 7]);

        // Side faces
        mesh.indices
            .push([base_index, base_index + 1, base_index + 5]);
        mesh.indices
            .push([base_index, base_index + 5, base_index + 4]);

        mesh.indices
            .push([base_index + 1, base_index + 2, base_index + 6]);
        mesh.indices
            .push([base_index + 1, base_index + 6, base_index + 5]);

        mesh.indices
            .push([base_index + 2, base_index + 3, base_index + 7]);
        mesh.indices
            .push([base_index + 2, base_index + 7, base_index + 6]);

        mesh.indices
            .push([base_index + 3, base_index, base_index + 4]);
        mesh.indices
            .push([base_index + 3, base_index + 4, base_index + 7]);
    }
}

impl Default for Turtle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsystem_expansion() {
        let mut lsys = LSystem::new("A");
        lsys.add_rule('A', "AB");
        lsys.add_rule('B', "A");

        let iter_0 = lsys.expand(0).unwrap();
        assert_eq!(iter_0, "A");

        let iter_1 = lsys.expand(1).unwrap();
        assert_eq!(iter_1, "AB");

        let iter_2 = lsys.expand(2).unwrap();
        assert_eq!(iter_2, "ABA");

        let iter_3 = lsys.expand(3).unwrap();
        assert_eq!(iter_3, "ABAAB");

        let iter_4 = lsys.expand(4).unwrap();
        assert_eq!(iter_4, "ABAABABA");
    }

    #[test]
    fn test_oom_prevention() {
        let mut lsys = LSystem::new("A");
        lsys.add_rule('A', "AAAAA"); // Exponential growth
        lsys.set_max_capacity(100);

        // 3 iterations: A -> 5 -> 25 -> 125 (should fail)
        assert!(lsys.expand(3).is_err());
    }

    #[test]
    #[ignore = "👹 Havoc: Intentionally tests an OOM vulnerability"]
    fn test_lsystem_havoc_oom() {
        // 👹 Havoc: Intentionally tests an Out-Of-Memory (OOM) vulnerability by creating a
        // runaway L-System. Handled gracefully by the system returning an Err now,
        // but kept as an ignored test to demonstrate fragility and limit checking.
        let mut lsys = LSystem::new("A");
        lsys.add_rule('A', "AAAAAAAAAA"); // 10x growth per iteration
        lsys.set_max_capacity(usize::MAX); // No limit!

        // This will attempt to allocate 10^15 characters, causing an OOM abort.
        let _ = lsys.expand(15);
    }

    #[test]
    fn test_turtle_mesh() {
        let mut turtle = Turtle::new();
        // A simple branch: push state, move forward, pop state, move forward
        let mesh = turtle.generate_mesh("[F]F").unwrap();

        // Each 'F' command generates a branch segment with 8 vertices and 12 triangles (36 indices)
        // Two 'F's mean 16 vertices and 72 indices
        assert_eq!(mesh.vertices.len(), 16);
        assert_eq!(mesh.indices.len(), 24); // 24 triangles * 3 = 72 indices in total length, wait, length is number of triangles. 12 triangles per segment * 2 = 24.
        assert_eq!(mesh.normals.len(), 16);
    }
}
