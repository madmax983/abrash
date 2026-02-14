use crate::math::{Vec2, Vec3, Vec4};

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

impl Lerp for Vec4 {
    fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
            w: self.w + (other.w - self.w) * t,
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

// For ((Vec3, f32), Vec2, Vec3, Vec4) - Pos+W, UV, Normal, Tangent
impl Lerp for ((Vec3, f32), Vec2, Vec3, Vec4) {
    fn lerp(self, other: Self, t: f32) -> Self {
        (
            self.0.lerp(other.0, t),
            self.1.lerp(other.1, t),
            self.2.lerp(other.2, t),
            self.3.lerp(other.3, t),
        )
    }
}

// For ((Vec3, f32), Vec3, Vec3) - Pos+W, Normal, WorldPos
impl Lerp for ((Vec3, f32), Vec3, Vec3) {
    fn lerp(self, other: Self, t: f32) -> Self {
        (
            self.0.lerp(other.0, t),
            self.1.lerp(other.1, t),
            self.2.lerp(other.2, t),
        )
    }
}

pub struct ClippedTriangles<V> {
    pub tris: [V; 24], // Max 8 triangles = 24 vertices
    pub count: usize,  // Number of triangles
}

/// Clip a triangle against the view frustum (6 planes) in Homogeneous Clip Space.
///
/// Returns a list of triangles (fan triangulation of the clipped polygon).
pub fn clip_triangle_to_frustum<V: Lerp + Copy>(
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

    // Unrolled inside mask check
    let mut m0 = 0;
    if p0.x >= -w0 {
        m0 |= 1;
    }
    if p0.x <= w0 {
        m0 |= 2;
    }
    if p0.y >= -w0 {
        m0 |= 4;
    }
    if p0.y <= w0 {
        m0 |= 8;
    }
    if p0.z >= -w0 {
        m0 |= 16;
    }
    if p0.z <= w0 {
        m0 |= 32;
    }

    let mut m1 = 0;
    if p1.x >= -w1 {
        m1 |= 1;
    }
    if p1.x <= w1 {
        m1 |= 2;
    }
    if p1.y >= -w1 {
        m1 |= 4;
    }
    if p1.y <= w1 {
        m1 |= 8;
    }
    if p1.z >= -w1 {
        m1 |= 16;
    }
    if p1.z <= w1 {
        m1 |= 32;
    }

    let mut m2 = 0;
    if p2.x >= -w2 {
        m2 |= 1;
    }
    if p2.x <= w2 {
        m2 |= 2;
    }
    if p2.y >= -w2 {
        m2 |= 4;
    }
    if p2.y <= w2 {
        m2 |= 8;
    }
    if p2.z >= -w2 {
        m2 |= 16;
    }
    if p2.z <= w2 {
        m2 |= 32;
    }

    let all_in = m0 & m1 & m2;
    if all_in == 0x3F {
        // Trivial Accept: All inside
        let mut tris = [v0; 24]; // Init with v0
        tris[0] = v0;
        tris[1] = v1;
        tris[2] = v2;
        return ClippedTriangles { tris, count: 1 };
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
    let mut buf1 = [v0; 12];
    let mut buf2 = [v0; 12];

    // Initialize input buffer
    buf1[0] = v0;
    buf1[1] = v1;
    buf1[2] = v2;
    let mut count = 3;

    // Macro to handle clipping logic for a plane
    // Reads from $buf_in, writes to $buf_out
    macro_rules! clip_plane {
        ($buf_in:ident, $buf_out:ident, $dist_fn:expr) => {
            if count > 0 {
                let mut out_count = 0;
                let prev_idx = count - 1;
                let mut prev_v = $buf_in[prev_idx];
                let (prev_pos, prev_w) = get_pos(&prev_v);
                let mut prev_d = $dist_fn(prev_pos, prev_w);

                for i in 0..count {
                    let curr_v = $buf_in[i];
                    let (curr_pos, curr_w) = get_pos(&curr_v);
                    let curr_d = $dist_fn(curr_pos, curr_w);

                    if curr_d >= 0.0 {
                        // Current is inside
                        if prev_d < 0.0 {
                            // Entered: add intersection
                            let t = prev_d / (prev_d - curr_d);
                            if out_count < 12 {
                                $buf_out[out_count] = prev_v.lerp(curr_v, t);
                                out_count += 1;
                            }
                        }
                        // Add current
                        if out_count < 12 {
                            $buf_out[out_count] = curr_v;
                            out_count += 1;
                        }
                    } else {
                        // Current is outside
                        if prev_d >= 0.0 {
                            // Exited: add intersection
                            let t = prev_d / (prev_d - curr_d);
                            if out_count < 12 {
                                $buf_out[out_count] = prev_v.lerp(curr_v, t);
                                out_count += 1;
                            }
                        }
                    }

                    prev_v = curr_v;
                    prev_d = curr_d;
                }
                count = out_count;
            }
        };
    }

    // Unroll loop over 6 planes using ping-pong buffering
    // 1. Left: x >= -w -> x + w >= 0
    clip_plane!(buf1, buf2, |p: Vec3, w: f32| p.x + w);

    // 2. Right: x <= w -> w - x >= 0
    clip_plane!(buf2, buf1, |p: Vec3, w: f32| w - p.x);

    // 3. Bottom: y >= -w -> y + w >= 0
    clip_plane!(buf1, buf2, |p: Vec3, w: f32| p.y + w);

    // 4. Top: y <= w -> w - y >= 0
    clip_plane!(buf2, buf1, |p: Vec3, w: f32| w - p.y);

    // 5. Near: z >= -w -> z + w >= 0
    clip_plane!(buf1, buf2, |p: Vec3, w: f32| p.z + w);

    // 6. Far: z <= w -> w - z >= 0
    clip_plane!(buf2, buf1, |p: Vec3, w: f32| w - p.z);

    // Result is in buf1 (since we did an even number of ping-pongs)

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

/// Clip a line segment against the view frustum (6 planes) in Homogeneous Clip Space.
///
/// Returns `Some((v0, v1))` if the line is partially or fully visible, `None` if fully culled.
pub fn clip_line_to_frustum<V: Lerp + Copy>(
    v0: V,
    v1: V,
    get_pos: impl Fn(&V) -> (Vec3, f32),
) -> Option<(V, V)> {
    let mut curr_v0 = v0;
    let mut curr_v1 = v1;

    // Helper macro to clip against a single plane
    // If a point is outside (dist < 0), we find the intersection.
    macro_rules! clip_plane {
        ($dist_fn:expr) => {
            let (p0, w0) = get_pos(&curr_v0);
            let (p1, w1) = get_pos(&curr_v1);
            let d0 = $dist_fn(p0, w0);
            let d1 = $dist_fn(p1, w1);

            if d0 >= 0.0 && d1 >= 0.0 {
                // Both inside, do nothing
            } else if d0 < 0.0 && d1 < 0.0 {
                // Both outside, cull entire line
                return None;
            } else {
                // One in, one out. Clip.
                let t = d0 / (d0 - d1);
                let intersection = curr_v0.lerp(curr_v1, t);

                if d0 < 0.0 {
                    // v0 is outside, replace v0
                    curr_v0 = intersection;
                } else {
                    // v1 is outside, replace v1
                    curr_v1 = intersection;
                }
            }
        };
    }

    // 1. Left: x >= -w -> x + w >= 0
    clip_plane!(|p: Vec3, w: f32| p.x + w);

    // 2. Right: x <= w -> w - x >= 0
    clip_plane!(|p: Vec3, w: f32| w - p.x);

    // 3. Bottom: y >= -w -> y + w >= 0
    clip_plane!(|p: Vec3, w: f32| p.y + w);

    // 4. Top: y <= w -> w - y >= 0
    clip_plane!(|p: Vec3, w: f32| w - p.y);

    // 5. Near: z >= -w -> z + w >= 0
    clip_plane!(|p: Vec3, w: f32| p.z + w);

    // 6. Far: z <= w -> w - z >= 0
    clip_plane!(|p: Vec3, w: f32| w - p.z);

    Some((curr_v0, curr_v1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    // Use (Vec3, f32) as vertex type where f32 is w
    type Vertex = (Vec3, f32);

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
                assert!(
                    pos.x <= w + 0.001,
                    "Point x={} w={} violates Right Plane",
                    pos.x,
                    w
                );
                // Check Left Plane: x >= -w
                assert!(
                    pos.x >= -w - 0.001,
                    "Point x={} w={} violates Left Plane",
                    pos.x,
                    w
                );
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
    fn test_line_fully_inside() {
        // Line fully inside the frustum
        // -w <= x,y,z <= w
        let w = 10.0;
        let v0: Vertex = (Vec3::new(0.0, 0.0, 0.0), w);
        let v1: Vertex = (Vec3::new(1.0, 1.0, 5.0), w);

        let result = clip_line_to_frustum(v0, v1, get_pos);

        assert!(result.is_some());
        let (r0, r1) = result.unwrap();
        assert_eq!(r0, v0);
        assert_eq!(r1, v1);
    }

    #[test]
    fn test_line_fully_outside() {
        // Line fully outside the frustum (to the right)
        // x > w
        let w = 10.0;
        let v0: Vertex = (Vec3::new(20.0, 0.0, 0.0), w);
        let v1: Vertex = (Vec3::new(25.0, 0.0, 0.0), w);

        let result = clip_line_to_frustum(v0, v1, get_pos);

        assert!(result.is_none());
    }

    #[test]
    fn test_line_intersects_one_plane() {
        // Line crossing the Right plane (x = w)
        // v0 inside: (5, 0, 0, 10) -> 5 <= 10
        // v1 outside: (15, 0, 0, 10) -> 15 > 10
        // Intersection should be at x=10
        let w = 10.0;
        let v0: Vertex = (Vec3::new(5.0, 0.0, 0.0), w);
        let v1: Vertex = (Vec3::new(15.0, 0.0, 0.0), w);

        let result = clip_line_to_frustum(v0, v1, get_pos);

        assert!(result.is_some());
        let (r0, r1) = result.unwrap();

        // v0 should be unchanged
        assert_eq!(r0, v0);

        // r1 should be on the boundary x=w=10
        // Interpolation: (15-5) range. 5 is 5 units from 10. 15 is 5 units from 10. Midpoint.
        // t = 0.5.
        // r1 pos = lerp(5, 15, 0.5) = 10.
        assert!((r1.0.x - 10.0).abs() < 1e-6);
        assert!((r1.0.y - 0.0).abs() < 1e-6);
        assert!((r1.0.z - 0.0).abs() < 1e-6);
        assert!((r1.1 - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_line_spanning_frustum() {
        // Line crossing Left and Right planes
        // v0: (-20, 0, 0, 10) -> Outside Left
        // v1: ( 20, 0, 0, 10) -> Outside Right
        let w = 10.0;
        let v0: Vertex = (Vec3::new(-20.0, 0.0, 0.0), w);
        let v1: Vertex = (Vec3::new(20.0, 0.0, 0.0), w);

        let result = clip_line_to_frustum(v0, v1, get_pos);

        assert!(result.is_some());
        let (r0, r1) = result.unwrap();

        // r0 should be at x = -10
        assert!((r0.0.x - (-10.0)).abs() < 1e-6);
        // r1 should be at x = 10
        assert!((r1.0.x - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_line_on_boundary() {
        // Line lying exactly on the Near plane boundary (z = -w)
        let w = 10.0;
        let v0: Vertex = (Vec3::new(0.0, 0.0, -10.0), w);
        let v1: Vertex = (Vec3::new(0.0, 5.0, -10.0), w);

        let result = clip_line_to_frustum(v0, v1, get_pos);

        assert!(result.is_some());
        let (r0, r1) = result.unwrap();

        assert_eq!(r0, v0);
        assert_eq!(r1, v1);
    }
}
