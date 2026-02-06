use crate::math::Lerp;

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
