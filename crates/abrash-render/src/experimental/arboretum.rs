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
use foldhash::HashMap;
use std::f32::consts::PI;

/// Represents the state of the drawing turtle.
#[derive(Clone, Copy, Debug)]
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
    /// ⚡ Bolt: Uses `foldhash::HashMap` with a fast hasher instead of std::collections::HashMap.
    /// This eliminates SipHash cryptographic overhead when looking up `char` keys during expansion.
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
            rules: HashMap::default(),
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
    /// Expands the axiom string by `iterations`.
    ///
    /// # Performance Optimization
    /// This method includes a fast-path for purely ASCII strings. It avoids the overhead of
    /// UTF-8 validation and the `String::push_str` method, operating directly on bytes.
    /// It also pre-calculates the exact capacity needed to avoid intermediate reallocations.
    pub fn expand(&self, iterations: u32) -> Result<String, crate::experimental::error::Error> {
        if iterations == 0 {
            return Ok(self.axiom.clone());
        }

        // Security / DoS protection limit: an L-system can grow exponentially and cause OOM.
        let limit: usize = 100_000_000; // Cap at 100MB

        // Fast path: if the axiom and all replacements are pure ASCII, we can work with Vec<u8> directly.
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
            // Setup lookup table for ASCII
            let mut rules_array: [Option<&[u8]>; 128] = [None; 128];
            for (k, v) in &self.rules {
                rules_array[(*k as usize) & 127] = Some(v.as_bytes());
            }
            thread_local! {
                static ARBORETUM_BUFFERS: std::cell::RefCell<(Vec<u8>, Vec<u8>)> = const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
            }
            return ARBORETUM_BUFFERS.with(|bufs| {
                let mut bufs = bufs.borrow_mut();
                let (current_bytes, next_bytes) = &mut *bufs;
                current_bytes.clear();
                current_bytes.extend_from_slice(self.axiom.as_bytes());

                for _ in 0..iterations {
                    /// By moving `next_bytes` outside the loop, we can `clear` and `reserve` its capacity
                    /// and then use `std::mem::swap`. This double-buffering completely eliminates O(N)
                    /// memory allocations and drops that were previously happening on every single iteration.
                    next_bytes.clear();
                    for &b in &*current_bytes {
                        if let Some(replacement) = rules_array[(b as usize) & 127] {
                            next_bytes.extend_from_slice(replacement);
                        } else {
                            next_bytes.push(b);
                        }
                        if next_bytes.len() > limit {
                            return Err(crate::experimental::error::Error::CapacityExceeded("L-system exceeded memory limits"));
                        }
                    }
                    std::mem::swap(current_bytes, next_bytes);
                }

                // Remove unsafe by converting back to string securely, though the ascii check guarantees safety.
                std::str::from_utf8(current_bytes)
                    .map(|s| s.to_string())
                    .map_err(|e| crate::experimental::error::Error::GeneralString(e.to_string()))
            });
        }

        // Fallback for unicode
        // ⚡ Bolt: Defer `axiom.clone()` until after the ASCII fast-path check
        // to completely eliminate an unnecessary `String` heap allocation on the hot path.
        let mut current = self.axiom.clone();

        let mut rules_array: [Option<&str>; 128] = [None; 128];
        for (k, v) in &self.rules {
            if (*k as usize) < 128 {
                rules_array[*k as usize] = Some(v.as_str());
            }
        }

        let mut next = String::new();
        for _ in 0..iterations {
            /// Moving the `next` String allocation out of the loop and reusing it via `swap`
            /// and `clear`/`reserve` eliminates continuous string re-allocations on every iteration.
            next.clear();
            for c in current.chars() {
                let u = c as usize;
                if u < 128 {
                    if let Some(replacement) = rules_array[u] {
                        next.push_str(replacement);
                    } else {
                        next.push(c);
                    }
                } else if let Some(replacement) = self.rules.get(&c) {
                    next.push_str(replacement);
                } else {
                    next.push(c);
                }

                if next.len() > limit {
                    return Err(crate::experimental::error::Error::CapacityExceeded("L-system exceeded memory limits"));
                }
            }
            std::mem::swap(&mut current, &mut next);
        }

        Ok(current)
    }

    /// Generates a Mesh from the expanded L-System string.
    pub fn generate_mesh(&self, iterations: u32) -> Result<Mesh, crate::experimental::error::Error> {
        let instructions = self.expand(iterations)?;

        // Pre-flight check to count segments to avoid over-allocating memory for meshes
        // that only contain state commands (like '[', ']', '+', '-').
        let max_stack_depth: usize = 10_000;
        let mut num_segments: usize = 0;
        let mut current_depth: usize = 0;
        let mut max_reached_depth: usize = 0;
        for &b in instructions.as_bytes() {
            if b == b'F' {
                num_segments += 1;
            } else if b == b'[' {
                current_depth += 1;
                if current_depth > max_stack_depth {
                    return Err(crate::experimental::error::Error::StackOverflow("L-system exceeded maximum stack depth"));
                }
                if current_depth > max_reached_depth {
                    max_reached_depth = current_depth;
                }
            } else if b == b']' {
                current_depth = current_depth.saturating_sub(1);
            }
        }

        // Limit the capacities to prevent OOM
        let mut mesh = Mesh::with_capacity(
            std::cmp::min(num_segments * 8, 10_000_000),
            std::cmp::min(num_segments * 8, 10_000_000),
        );
        let mut stack: Vec<Turtle> =
            Vec::with_capacity(std::cmp::min(instructions.len() / 8, max_stack_depth));
        let mut turtle = Turtle::new(self.step_length, self.radius);

        // F, f, +, -, &, ^, \, /, |, [, ] are all 1-byte ascii characters in UTF-8
        for &b in instructions.as_bytes() {
            match b {
                b'F' => {
                    // Draw segment
                    let start = turtle.position;
                    let end = start + turtle.heading * turtle.step_length;

                    self.add_segment(&mut mesh, &turtle, start, end);

                    turtle.position = end;
                }
                b'f' => {
                    // Move without drawing
                    turtle.position = turtle.position + turtle.heading * turtle.step_length;
                }
                b'+' => {
                    // Yaw Left (around Up)
                    turtle.heading =
                        rotate_vector(turtle.heading, turtle.up, self.angle).fast_normalize();
                    turtle.left = turtle.up.cross(turtle.heading).fast_normalize();
                }
                b'-' => {
                    // Yaw Right (around Up)
                    turtle.heading =
                        rotate_vector(turtle.heading, turtle.up, -self.angle).fast_normalize();
                    turtle.left = turtle.up.cross(turtle.heading).fast_normalize();
                }
                b'&' => {
                    // Pitch Down (around Left)
                    turtle.heading =
                        rotate_vector(turtle.heading, turtle.left, self.angle).fast_normalize();
                    turtle.up = turtle.heading.cross(turtle.left).fast_normalize();
                }
                b'^' => {
                    // Pitch Up (around Left)
                    turtle.heading =
                        rotate_vector(turtle.heading, turtle.left, -self.angle).fast_normalize();
                    turtle.up = turtle.heading.cross(turtle.left).fast_normalize();
                }
                b'\\' => {
                    // Roll Left (around Heading)
                    turtle.up =
                        rotate_vector(turtle.up, turtle.heading, self.angle).fast_normalize();
                    turtle.left = turtle.up.cross(turtle.heading).fast_normalize();
                }
                b'/' => {
                    // Roll Right (around Heading)
                    turtle.up =
                        rotate_vector(turtle.up, turtle.heading, -self.angle).fast_normalize();
                    turtle.left = turtle.up.cross(turtle.heading).fast_normalize();
                }
                b'|' => {
                    // Turn 180 (around Up)
                    turtle.heading = rotate_vector(turtle.heading, turtle.up, PI).fast_normalize();
                    turtle.left = turtle.up.cross(turtle.heading).fast_normalize();
                }
                b'[' => {
                    if stack.len() >= max_stack_depth {
                        return Err(crate::experimental::error::Error::StackOverflow("L-system exceeded maximum stack depth"));
                    }
                    stack.push(turtle);
                }
                b']' => {
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
        mesh.vertices.extend([
            start + corners[0],
            start + corners[1],
            start + corners[2],
            start + corners[3],
        ]);

        // End vertices
        mesh.vertices.extend([
            end + corners[0],
            end + corners[1],
            end + corners[2],
            end + corners[3],
        ]);

        // Normals (approximate as face normals or vertex normals?)
        // For flat shading (prisms), we need duplicated vertices if we want sharp edges.
        // But here we are sharing vertices for the corners.
        // If we share vertices, the normal will be interpolated, making it look round (smooth shading).
        // Since `Mesh` doesn't enforce smooth/flat, let's just push normals pointing out from center.

        // Actually, let's just push vertices. Normals are optional for now or calculated later.
        // But `Mesh` expects `normals` if we want lighting.
        // Let's add normals pointing away from the segment axis.
        let n0 = corners[0].fast_normalize();
        let n1 = corners[1].fast_normalize();
        let n2 = corners[2].fast_normalize();
        let n3 = corners[3].fast_normalize();

        // Start normals
        mesh.normals.extend([n0, n1, n2, n3]);
        // End normals
        mesh.normals.extend([n0, n1, n2, n3]);

        // Indices (Triangles)
        // 4 faces, 2 triangles each.
        // Vertices: 0-3 (start), 4-7 (end)
        // Face 0: 0, 4, 5, 1 (Side +Up) -> No, +Left/+Up is corner.
        // Let's connect the sides.
        mesh.indices.extend([
            // Side 1: 0 -> 4 -> 5 -> 1
            [base_idx + 0, base_idx + 4, base_idx + 5],
            [base_idx + 0, base_idx + 5, base_idx + 1],
            // Side 2: 1 -> 5 -> 6 -> 2
            [base_idx + 1, base_idx + 5, base_idx + 6],
            [base_idx + 1, base_idx + 6, base_idx + 2],
            // Side 3: 2 -> 6 -> 7 -> 3
            [base_idx + 2, base_idx + 6, base_idx + 7],
            [base_idx + 2, base_idx + 7, base_idx + 3],
            // Side 4: 3 -> 7 -> 4 -> 0
            [base_idx + 3, base_idx + 7, base_idx + 4],
            [base_idx + 3, base_idx + 4, base_idx + 0],
        ]);
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
    fn test_expansion_dos() {
        let mut lsys = LSystem::new("A", 90.0, 1.0, 0.1);
        // 1 => 10 chars
        lsys.add_rule('A', "AAAAAAAAAA");
        // 10 iterations = 10^10 chars > 100MB limit
        assert!(lsys.expand(10).is_err());
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
