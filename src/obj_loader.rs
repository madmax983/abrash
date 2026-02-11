//! Wavefront OBJ file loader.
//!
//! This module provides a simple parser for `.obj` files.
//!
//! # Supported Features
//!
//! *   **Vertices (`v`)**: 3D positions (x, y, z).
//! *   **Texture Coordinates (`vt`)**: 2D UVs (u, v).
//! *   **Faces (`f`)**: Triangles and Quads (automatically triangulated).
//!     *   Supports `v`, `v/vt`, `v//vn`, and `v/vt/vn` formats.
//!
//! # Limitations
//!
//! *   **Normals (`vn`)**: Parsed but currently ignored/discarded.
//! *   **Materials (`usemtl`, `mtllib`)**: Ignored.
//! *   **Groups (`g`, `o`)**: Ignored.

use crate::math::{Vec2, Vec3};
use crate::mesh::Mesh;

struct CacheNode {
    vt_idx: usize, // usize::MAX if None
    new_idx: usize,
    next: usize, // usize::MAX if None
}

/// Load a Mesh from a Wavefront OBJ string source.
///
/// # Examples
///
/// ```
/// use abrash::obj_loader::load_obj;
///
/// let obj_source = "
/// v 0.0 0.0 0.0
/// v 1.0 0.0 0.0
/// v 0.0 1.0 0.0
/// f 1 2 3
/// ";
///
/// let mesh = load_obj(obj_source).unwrap();
/// assert_eq!(mesh.vertices.len(), 3);
/// ```
#[allow(clippy::missing_errors_doc)]
pub fn load_obj(source: &str) -> Result<Mesh, String> {
    // Reserve reasonable initial capacity to avoid frequent reallocations
    let mut raw_positions = Vec::with_capacity(1024);
    let mut raw_uvs = Vec::with_capacity(1024);

    // Deduplication structure:
    // We replace the standard HashMap with a custom separate-chaining lookup table
    // indexed directly by vertex index (v_idx). This avoids hashing overhead and
    // takes advantage of the fact that unique `(v_idx, vt_idx)` pairs are sparse
    // but clustered by `v_idx`.

    // Head of the chain for each v_idx. Stores index into `cache_nodes`.
    // We initialize/grow this parallel to `raw_positions`.
    // value usize::MAX indicates "None".
    let mut cache_head: Vec<usize> = Vec::with_capacity(1024);

    // Nodes in the chains.
    let mut cache_nodes: Vec<CacheNode> = Vec::with_capacity(1024);

    let mut final_vertices = Vec::with_capacity(1024);
    let mut final_uvs = Vec::with_capacity(1024);
    let mut final_indices = Vec::with_capacity(1024);

    // Reuse vector for face indices to avoid allocation per face
    let mut face_indices = Vec::with_capacity(4);

    for (line_num, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let cmd = parts.next().unwrap_or("");

        match cmd {
            "v" => {
                let x = parts
                    .next()
                    .ok_or_else(|| format!("Line {line_num}: Missing x"))?
                    .parse::<f32>()
                    .map_err(|_| format!("Line {line_num}: Invalid x"))?;
                let y = parts
                    .next()
                    .ok_or_else(|| format!("Line {line_num}: Missing y"))?
                    .parse::<f32>()
                    .map_err(|_| format!("Line {line_num}: Invalid y"))?;
                let z = parts
                    .next()
                    .ok_or_else(|| format!("Line {line_num}: Missing z"))?
                    .parse::<f32>()
                    .map_err(|_| format!("Line {line_num}: Invalid z"))?;

                if !x.is_finite() || !y.is_finite() || !z.is_finite() {
                    return Err(format!("Line {line_num}: Coordinates must be finite"));
                }
                raw_positions.push(Vec3::new(x, y, z));
                // Grow cache_head to match raw_positions
                cache_head.push(usize::MAX);
            }
            "vt" => {
                let u = parts
                    .next()
                    .ok_or_else(|| format!("Line {line_num}: Missing u"))?
                    .parse::<f32>()
                    .map_err(|_| format!("Line {line_num}: Invalid u"))?;
                let v = parts
                    .next()
                    .ok_or_else(|| format!("Line {line_num}: Missing v"))?
                    .parse::<f32>()
                    .map_err(|_| format!("Line {line_num}: Invalid v"))?;

                if !u.is_finite() || !v.is_finite() {
                    return Err(format!("Line {line_num}: UV coordinates must be finite"));
                }
                raw_uvs.push(Vec2::new(u, v));
            }
            "f" => {
                face_indices.clear();
                for part in parts {
                    // format: v, v/vt, v//vn, v/vt/vn
                    let mut segs = part.split('/');

                    // Position index
                    let v_str = segs
                        .next()
                        .ok_or_else(|| format!("Line {line_num}: Invalid face format"))?;
                    let v_idx = v_str
                        .parse::<usize>()
                        .map_err(|_| format!("Line {line_num}: Invalid vertex index"))?;
                    // OBJ is 1-based
                    let v_idx = v_idx
                        .checked_sub(1)
                        .ok_or_else(|| format!("Line {line_num}: Vertex index 0 is invalid"))?;

                    // UV index
                    let vt_idx = if let Some(vt_str) = segs.next().filter(|s| !s.is_empty()) {
                        let idx = vt_str
                            .parse::<usize>()
                            .map_err(|_| format!("Line {line_num}: Invalid UV index"))?;
                        Some(
                            idx.checked_sub(1)
                                .ok_or_else(|| format!("Line {line_num}: UV index 0 is invalid"))?,
                        )
                    } else {
                        None
                    };

                    // Look up or insert
                    if v_idx >= raw_positions.len() {
                        return Err(format!(
                            "Line {}: Vertex index {} out of bounds",
                            line_num,
                            v_idx + 1
                        ));
                    }

                    // Key for lookup
                    let vt_key = vt_idx.unwrap_or(usize::MAX);

                    // Linear scan in the cache chain for this vertex
                    let mut found_idx = None;
                    let mut curr = cache_head[v_idx];
                    while curr != usize::MAX {
                        let node = &cache_nodes[curr];
                        if node.vt_idx == vt_key {
                            found_idx = Some(node.new_idx);
                            break;
                        }
                        curr = node.next;
                    }

                    if let Some(idx) = found_idx {
                        face_indices.push(idx);
                    } else {
                        let new_idx = final_vertices.len();

                        // Push vertex
                        final_vertices.push(raw_positions[v_idx]);

                        // Push UV (or default 0,0)
                        if let Some(ti) = vt_idx {
                            if ti >= raw_uvs.len() {
                                return Err(format!(
                                    "Line {}: UV index {} out of bounds",
                                    line_num,
                                    ti + 1
                                ));
                            }
                            final_uvs.push(raw_uvs[ti]);
                        } else {
                            final_uvs.push(Vec2::new(0.0, 0.0));
                        }

                        // Insert into cache
                        let new_node_idx = cache_nodes.len();
                        cache_nodes.push(CacheNode {
                            vt_idx: vt_key,
                            new_idx,
                            next: cache_head[v_idx],
                        });
                        cache_head[v_idx] = new_node_idx;

                        face_indices.push(new_idx);
                    }
                }

                // Triangulate fan
                if face_indices.len() < 3 {
                    return Err(format!("Line {line_num}: Face has fewer than 3 vertices"));
                }

                for i in 1..face_indices.len() - 1 {
                    final_indices.push([face_indices[0], face_indices[i], face_indices[i + 1]]);
                }
            }
            _ => {} // Ignore normals (vn), groups (g), materials (usemtl), etc.
        }
    }

    Ok(Mesh {
        vertices: final_vertices,
        indices: final_indices,
        uvs: final_uvs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_cube() {
        let obj = "
v -1.0 -1.0 1.0
v 1.0 -1.0 1.0
v 1.0 1.0 1.0
v -1.0 1.0 1.0
f 1 2 3
f 1 3 4
";
        let mesh = load_obj(obj).unwrap();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.indices.len(), 2);

        // Check positions
        assert_eq!(mesh.vertices[0], Vec3::new(-1.0, -1.0, 1.0));
    }

    #[test]
    fn test_load_with_uvs() {
        let obj = "
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
f 1/1 2/2 3/3
";
        let mesh = load_obj(obj).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.uvs.len(), 3);
        assert_eq!(mesh.uvs[0], Vec2::new(0.0, 0.0));
        assert_eq!(mesh.uvs[1], Vec2::new(1.0, 0.0));
    }

    #[test]
    fn test_deduplication() {
        // Vertex 1 used twice with same UV
        let obj = "
v 0 0 0
v 1 0 0
v 0 1 0
f 1 2 3
f 1 3 2
";
        let mesh = load_obj(obj).unwrap();
        // Should only have 3 vertices, even though referenced multiple times
        assert_eq!(mesh.vertices.len(), 3);
    }

    #[test]
    fn test_split_vertices() {
        // Vertex 1 used with different UVs should split
        let obj = "
v 0 0 0
vt 0 0
vt 1 1
f 1/1 1/2 1/1
";
        let mesh = load_obj(obj).unwrap();
        // Vertex 1 is used with vt 1 and vt 2.
        // vt 1 is used twice.
        // So we expect 2 unique vertices.
        assert_eq!(mesh.vertices.len(), 2);
    }
}
