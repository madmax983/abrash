use crate::math::{Vec2, Vec3};

pub trait Lerp: Copy + Clone {
    #[must_use]
    fn lerp(self, other: Self, t: f32) -> Self;
}

// Implement Lerp for primitive types

impl Lerp for f32 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Lerp for Vec2 {
    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

impl Lerp for Vec3 {
    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }
}

// For (Vec3, f32) - Position and W
impl Lerp for (Vec3, f32) {
    fn lerp(self, other: Self, t: f32) -> Self {
        (self.0.lerp(other.0, t), self.1.lerp(other.1, t))
    }
}

// For ((Vec3, f32), Vec3) - Pos+W, Color
impl Lerp for ((Vec3, f32), Vec3) {
    fn lerp(self, other: Self, t: f32) -> Self {
        (self.0.lerp(other.0, t), self.1.lerp(other.1, t))
    }
}

// For ((Vec3, f32), Vec2) - Pos+W, UV
impl Lerp for ((Vec3, f32), Vec2) {
    fn lerp(self, other: Self, t: f32) -> Self {
        (self.0.lerp(other.0, t), self.1.lerp(other.1, t))
    }
}

pub struct ClippedTriangles<V> {
    pub tris: [V; 24], // Max 8 triangles = 24 vertices
    pub count: usize,  // Number of triangles
}

const NEAR: f32 = 0.001;

/// Clip a triangle against the view frustum (6 planes) in Homogeneous Clip Space.
///
/// Returns a list of triangles (fan triangulation of the clipped polygon).
pub fn clip_triangle_to_frustum<V: Lerp + Copy + Default>(
    v0: V,
    v1: V,
    v2: V,
    get_pos: impl Fn(&V) -> (Vec3, f32),
) -> ClippedTriangles<V> {
    // Optimization: Trivial Accept/Reject
    // Check if all vertices are inside all planes (Accept) or all outside one plane (Reject)
    let (p0, w0) = get_pos(&v0);
    let (p1, w1) = get_pos(&v1);
    let (p2, w2) = get_pos(&v2);

    let inside_mask = |p: Vec3, w: f32| -> u8 {
        let mut mask = 0;
        if p.x >= -w { mask |= 1; }
        if p.x <= w  { mask |= 2; }
        if p.y >= -w { mask |= 4; }
        if p.y <= w  { mask |= 8; }
        if p.z >= -w { mask |= 16; }
        if p.z <= w  { mask |= 32; }
        mask
    };

    let m0 = inside_mask(p0, w0);
    let m1 = inside_mask(p1, w1);
    let m2 = inside_mask(p2, w2);

    let all_in = m0 & m1 & m2;
    if all_in == 0x3F {
        // Trivial Accept: All inside
        let mut tris = [v0; 24]; // Init with v0
        tris[0] = v0;
        tris[1] = v1;
        tris[2] = v2;
        return ClippedTriangles {
            tris,
            count: 1,
        };
    }

    let any_in = m0 | m1 | m2;
    if any_in != 0x3F {
        // Trivial Reject: All outside at least one plane
        return ClippedTriangles {
            tris: [v0; 24],
            count: 0,
        };
    }

    // Double buffering for vertex lists
    // A triangle clipped by 6 planes can have at most 9 vertices (usually).
    // We use a safe upper bound of 12 for the polygon vertices.
    // The final triangulation can produce more vertices in the output structure.
    let mut buf1 = [v0; 12];
    let mut buf2 = [v0; 12];

    // Initialize input buffer
    buf1[0] = v0;
    buf1[1] = v1;
    buf1[2] = v2;
    let mut count = 3;

    // Define clip planes
    // Each plane is defined by a function that returns the signed distance to the plane.
    // Point is inside if distance >= 0.
    let planes: [fn(Vec3, f32) -> f32; 6] = [
        // Left: x >= -w -> x + w >= 0
        |p: Vec3, w: f32| p.x + w,
        // Right: x <= w -> w - x >= 0
        |p: Vec3, w: f32| w - p.x,
        // Bottom: y >= -w -> y + w >= 0
        |p: Vec3, w: f32| p.y + w,
        // Top: y <= w -> w - y >= 0
        |p: Vec3, w: f32| w - p.y,
        // Near: z >= -w -> z + w >= 0 (Standard OpenGL)
        // Note: Use a small epsilon for Near to avoid w=0 issues if needed,
        // but typically z+w is fine. The previous code used w >= 0.001.
        // Let's use z + w >= 0. But strictly speaking, we want to clip things behind the camera.
        // If we use z + w >= 0, that's z_ndc >= -1.
        |p: Vec3, w: f32| p.z + w,
        // Far: z <= w -> w - z >= 0
        |p: Vec3, w: f32| w - p.z,
    ];

    // Current buffer pointer (swapping logic)
    // We start reading from buf1, writing to buf2.
    // Then swap.
    // Since we can't easily return references to local variables, we just copy data.

    for plane in planes {
        if count == 0 {
            break;
        }

        let mut out_count = 0;

        // We always read from 'buf1' (conceptually) and write to 'buf2', then copy back.

        let prev_idx = count - 1;
        let mut prev_v = buf1[prev_idx];
        let (prev_pos, prev_w) = get_pos(&prev_v);
        let mut prev_d = plane(prev_pos, prev_w);

        for i in 0..count {
            let curr_v = buf1[i];
            let (curr_pos, curr_w) = get_pos(&curr_v);
            let curr_d = plane(curr_pos, curr_w);

            if curr_d >= 0.0 {
                // Current is inside
                if prev_d < 0.0 {
                    // Entered: add intersection
                    let t = prev_d / (prev_d - curr_d);
                    if out_count < 12 {
                        buf2[out_count] = prev_v.lerp(curr_v, t);
                        out_count += 1;
                    }
                }
                // Add current
                if out_count < 12 {
                    buf2[out_count] = curr_v;
                    out_count += 1;
                }
            } else {
                // Current is outside
                if prev_d >= 0.0 {
                    // Exited: add intersection
                    let t = prev_d / (prev_d - curr_d);
                    if out_count < 12 {
                        buf2[out_count] = prev_v.lerp(curr_v, t);
                        out_count += 1;
                    }
                }
            }

            prev_v = curr_v;
            prev_d = curr_d;
        }

        // Swap buffers (copy buf2 to buf1)
        count = out_count;
        buf1[..count].copy_from_slice(&buf2[..count]);
    }

    // Triangulate (Fan)
    let mut result = ClippedTriangles {
        tris: [v0; 24],
        count: 0,
    };

    if count >= 3 {
        // Pivot vertex
        let pivot = buf1[0];
        // Generate triangles: (0, 1, 2), (0, 2, 3), (0, 3, 4), ...
        // Number of triangles = count - 2

        for i in 1..count - 1 {
            if result.count < 8 {
                let idx = result.count * 3;
                result.tris[idx] = pivot;
                result.tris[idx + 1] = buf1[i];
                result.tris[idx + 2] = buf1[i + 1];
                result.count += 1;
            }
        }
    }

    result
}

// Keep the old function for now if needed, or deprecate.
pub fn clip_triangle_against_near_plane<V: Lerp>(
    v0: V,
    v1: V,
    v2: V,
    get_w: impl Fn(&V) -> f32,
) -> ClippedTriangles<V> {
    // Check which vertices are inside (w >= NEAR)
    let inside0 = get_w(&v0) >= NEAR;
    let inside1 = get_w(&v1) >= NEAR;
    let inside2 = get_w(&v2) >= NEAR;

    let inside_count = usize::from(inside0) + usize::from(inside1) + usize::from(inside2);

    // We initialize the array with v0 copies just to satisfy initialization.
    // They will be overwritten if used.
    let mut result = ClippedTriangles {
        tris: [v0; 24],
        count: 0,
    };

    if inside_count == 3 {
        // All inside - return original
        result.tris[0] = v0;
        result.tris[1] = v1;
        result.tris[2] = v2;
        result.count = 1;
        return result;
    }

    if inside_count == 0 {
        // All outside - cull
        return result;
    }

    // Clipping needed
    // Helper to intersect edge against w=NEAR
    let intersect = |start: V, end: V| -> V {
        let w_start = get_w(&start);
        let w_end = get_w(&end);
        let t = (NEAR - w_start) / (w_end - w_start);
        start.lerp(end, t)
    };

    // Logic depends on which are inside.
    let vertices = [v0, v1, v2];
    let inside = [inside0, inside1, inside2];

    let mut out_verts = [v0; 4]; // Max 4 vertices for a clipped triangle (quad)
    let mut out_count = 0;

    for i in 0..3 {
        let curr = vertices[i];
        let next = vertices[(i + 1) % 3];
        let curr_in = inside[i];
        let next_in = inside[(i + 1) % 3];

        if curr_in {
            out_verts[out_count] = curr;
            out_count += 1;
        }

        if curr_in != next_in {
            out_verts[out_count] = intersect(curr, next);
            out_count += 1;
        }
    }

    // Now assemble triangles
    if out_count == 3 {
        result.tris[0] = out_verts[0];
        result.tris[1] = out_verts[1];
        result.tris[2] = out_verts[2];
        result.count = 1;
    } else if out_count == 4 {
        // Quad (0,1,2,3) -> Tri1(0,1,2), Tri2(0,2,3)
        result.tris[0] = out_verts[0];
        result.tris[1] = out_verts[1];
        result.tris[2] = out_verts[2];

        result.tris[3] = out_verts[0];
        result.tris[4] = out_verts[2];
        result.tris[5] = out_verts[3];
        result.count = 2;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    // Use (Vec3, f32) as vertex type where f32 is w
    type Vertex = (Vec3, f32);

    fn get_w(v: &Vertex) -> f32 {
        v.1
    }

    fn get_pos(v: &Vertex) -> (Vec3, f32) {
        *v
    }

    #[test]
    fn test_frustum_clipping_right_plane() {
        // Triangle crossing the right plane (x = w)
        // v0: (-5, 0, 0, 10)  -> x/w = -0.5 (Inside)
        // v1: ( 5, 0, 0, 10)  -> x/w = 0.5 (Inside)
        // v2: (15, 0, 0, 10)  -> x/w = 1.5 (Outside)

        let v0: Vertex = (Vec3::new(-5.0, 0.0, 0.0), 10.0);
        let v1: Vertex = (Vec3::new(5.0, 0.0, 0.0), 10.0);
        let v2: Vertex = (Vec3::new(15.0, 0.0, 0.0), 10.0);

        let result = clip_triangle_to_frustum(v0, v1, v2, get_pos);

        // Should be clipped
        assert!(result.count > 0, "Should output at least one triangle");

        for i in 0..result.count {
             let base = i * 3;
             for k in 0..3 {
                 let v = result.tris[base + k];
                 let (pos, w) = v;
                 // Check Right Plane: x <= w
                 assert!(pos.x <= w + 0.001, "Point x={} w={} violates Right Plane", pos.x, w);
                 // Check Left Plane: x >= -w
                 assert!(pos.x >= -w - 0.001, "Point x={} w={} violates Left Plane", pos.x, w);
             }
        }
    }

    #[test]
    fn test_frustum_all_inside() {
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), 10.0);
        let v1: Vertex = (Vec3::new(1.0, 0.0, 0.0), 10.0);
        let v2: Vertex = (Vec3::new(0.0, 1.0, 0.0), 10.0);

        let result = clip_triangle_to_frustum(v0, v1, v2, get_pos);

        assert_eq!(result.count, 1);
        assert_eq!(result.tris[0], v0);
        assert_eq!(result.tris[1], v1);
        assert_eq!(result.tris[2], v2);
    }

    #[test]
    fn test_frustum_all_outside() {
        // All far to the right
        let v0: Vertex = (Vec3::new(20.0, 0.0, 0.0), 10.0);
        let v1: Vertex = (Vec3::new(21.0, 0.0, 0.0), 10.0);
        let v2: Vertex = (Vec3::new(20.0, 1.0, 0.0), 10.0);

        let result = clip_triangle_to_frustum(v0, v1, v2, get_pos);

        assert_eq!(result.count, 0);
    }

    #[test]
    fn test_all_inside() {
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), 1.0);
        let v1: Vertex = (Vec3::new(1.0, 0.0, 0.0), 1.0);
        let v2: Vertex = (Vec3::new(0.0, 1.0, 0.0), 1.0);

        let result = clip_triangle_against_near_plane(v0, v1, v2, get_w);

        assert_eq!(result.count, 1);
        assert_eq!(result.tris[0], v0);
        assert_eq!(result.tris[1], v1);
        assert_eq!(result.tris[2], v2);
    }

    #[test]
    fn test_all_outside() {
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), -1.0);
        let v1: Vertex = (Vec3::new(1.0, 0.0, 0.0), -1.0);
        let v2: Vertex = (Vec3::new(0.0, 1.0, 0.0), -1.0);

        let result = clip_triangle_against_near_plane(v0, v1, v2, get_w);

        assert_eq!(result.count, 0);
    }

    #[test]
    fn test_one_inside() {
        // v0 inside (w=1.0), v1, v2 outside (w=-1.0)
        let v0: Vertex = (Vec3::new(0.0, 0.0, 1.0), 1.0);
        let v1: Vertex = (Vec3::new(0.0, 2.0, -1.0), -1.0);
        let v2: Vertex = (Vec3::new(2.0, 0.0, -1.0), -1.0);

        let result = clip_triangle_against_near_plane(v0, v1, v2, get_w);

        // Should return 1 triangle
        assert_eq!(result.count, 1);

        // Check vertices
        // The first vertex should be v0
        assert_eq!(result.tris[0], v0);

        // The other two should have w = NEAR (0.001)
        assert!((result.tris[1].1 - NEAR).abs() < 1e-6);
        assert!((result.tris[2].1 - NEAR).abs() < 1e-6);
    }

    #[test]
    fn test_two_inside() {
        // v0, v1 inside (w=1.0), v2 outside (w=-1.0)
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), 1.0);
        let v1: Vertex = (Vec3::new(1.0, 0.0, 0.0), 1.0);
        let v2: Vertex = (Vec3::new(0.0, 2.0, -1.0), -1.0); // Outside

        let result = clip_triangle_against_near_plane(v0, v1, v2, get_w);

        // Should return 2 triangles (quad)
        assert_eq!(result.count, 2);

        // Verify structure of returned triangles
        // First triangle: v0, v1, intersect(v1, v2)
        // Second triangle: v0, intersect(v1, v2), intersect(v2, v0)

        // Check that we have vertices with w=NEAR
        // Count how many have w=NEAR
        // Total vertices in result = 3 * count = 6

        // Tri1: v0, v1, intersect(v1, v2) -> w: 1.0, 1.0, NEAR
        // Tri2: v0, intersect(v1, v2), intersect(v2, v0) -> w: 1.0, NEAR, NEAR

        // Total w values: 1.0, 1.0, NEAR, 1.0, NEAR, NEAR.
        // So 3 vertices with w=1.0, 3 vertices with w=NEAR.

        let mut near_count = 0;
        let mut inner_count = 0;

        for i in 0..6 {
            if (result.tris[i].1 - NEAR).abs() < 1e-6 {
                near_count += 1;
            } else if (result.tris[i].1 - 1.0).abs() < 1e-6 {
                inner_count += 1;
            }
        }

        assert_eq!(near_count, 3);
        assert_eq!(inner_count, 3);
    }

    #[test]
    fn test_exact_boundary() {
        // All vertices exactly on NEAR
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), NEAR);
        let v1: Vertex = (Vec3::new(1.0, 0.0, 0.0), NEAR);
        let v2: Vertex = (Vec3::new(0.0, 1.0, 0.0), NEAR);

        let result = clip_triangle_against_near_plane(v0, v1, v2, get_w);

        // Should be considered inside
        assert_eq!(result.count, 1);
    }

    #[test]
    fn test_epsilon_boundary() {
        // v0 just below NEAR, v1 just above
        let eps = 1e-7;
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), NEAR - eps); // Outside
        let v1: Vertex = (Vec3::new(1.0, 0.0, 0.0), NEAR + eps); // Inside
        let v2: Vertex = (Vec3::new(0.0, 1.0, 0.0), 1.0); // Inside

        let result = clip_triangle_against_near_plane(v0, v1, v2, get_w);

        // Should clip to 2 triangles (quad) since 2 are inside
        assert_eq!(result.count, 2);
    }
}
