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
    pub tris: [V; 6], // Max 2 triangles = 6 vertices
    pub count: usize, // Number of triangles (0, 1, or 2)
}

const NEAR: f32 = 0.001;

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
        tris: [v0; 6],
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
