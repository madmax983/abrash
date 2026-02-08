//! Wavefront OBJ 3D Model Loader.
//!
//! This module provides functionality to parse Wavefront OBJ files into a `Mesh` structure
//! suitable for rendering. It supports parsing vertices (`v`), texture coordinates (`vt`),
//! and faces (`f`).
//!
//! # Features
//!
//! *   **Vertex Deduplication**: Vertices with unique position/UV combinations are automatically
//!     deduplicated and indexed.
//! *   **Triangulation**: Faces with more than 3 vertices (polygons) are automatically triangulated
//!     using a triangle fan.
//! *   **Safety**: All parsing is done with safe Rust, including bounds checking.
//!
//! # Limitations
//!
//! *   Normals (`vn`) are currently ignored.
//! *   Materials (`mtllib`, `usemtl`) are ignored.
//! *   Groups (`g`, `o`) are ignored; the entire file is loaded as a single mesh.

use crate::math::{Vec2, Vec3};
use crate::mesh::Mesh;
use std::collections::HashMap;

/// Load a Mesh from a Wavefront OBJ string source.
///
/// This function parses a string containing OBJ data and returns a `Mesh` object.
///
/// # Arguments
///
/// * `source` - A string slice containing the OBJ file content.
///
/// # Returns
///
/// * `Ok(Mesh)` - The parsed mesh on success.
/// * `Err(String)` - An error message describing why parsing failed (e.g., malformed line, invalid index).
///
/// # Examples
///
/// ```
/// use abrash::obj_loader::load_obj;
/// use abrash::math::Vec3;
///
/// let obj_data = "
/// v -0.5 -0.5 0.0
/// v  0.5 -0.5 0.0
/// v  0.0  0.5 0.0
/// f 1 2 3
/// ";
///
/// let mesh = load_obj(obj_data).unwrap();
/// assert_eq!(mesh.vertices.len(), 3);
/// assert_eq!(mesh.indices.len(), 1);
/// assert_eq!(mesh.vertices[0], Vec3::new(-0.5, -0.5, 0.0));
/// ```
pub fn load_obj(source: &str) -> Result<Mesh, String> {
    // Reserve reasonable initial capacity to avoid frequent reallocations
    let mut raw_positions = Vec::with_capacity(1024);
    let mut raw_uvs = Vec::with_capacity(1024);

    // We need to deduplicate vertices.
    // Key: (position_index, uv_index) -> Value: new_index
    // position_index is required, uv_index is optional.
    let mut unique_vertices: HashMap<(usize, Option<usize>), usize> = HashMap::with_capacity(1024);

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
                    .ok_or_else(|| format!("Line {}: Missing x", line_num))?
                    .parse::<f32>()
                    .map_err(|_| format!("Line {}: Invalid x", line_num))?;
                let y = parts
                    .next()
                    .ok_or_else(|| format!("Line {}: Missing y", line_num))?
                    .parse::<f32>()
                    .map_err(|_| format!("Line {}: Invalid y", line_num))?;
                let z = parts
                    .next()
                    .ok_or_else(|| format!("Line {}: Missing z", line_num))?
                    .parse::<f32>()
                    .map_err(|_| format!("Line {}: Invalid z", line_num))?;
                raw_positions.push(Vec3::new(x, y, z));
            }
            "vt" => {
                let u = parts
                    .next()
                    .ok_or_else(|| format!("Line {}: Missing u", line_num))?
                    .parse::<f32>()
                    .map_err(|_| format!("Line {}: Invalid u", line_num))?;
                let v = parts
                    .next()
                    .ok_or_else(|| format!("Line {}: Missing v", line_num))?
                    .parse::<f32>()
                    .map_err(|_| format!("Line {}: Invalid v", line_num))?;
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
                        .ok_or_else(|| format!("Line {}: Invalid face format", line_num))?;
                    let v_idx = v_str
                        .parse::<usize>()
                        .map_err(|_| format!("Line {}: Invalid vertex index", line_num))?;
                    // OBJ is 1-based
                    let v_idx = v_idx
                        .checked_sub(1)
                        .ok_or_else(|| format!("Line {}: Vertex index 0 is invalid", line_num))?;

                    // UV index
                    let mut vt_idx = None;
                    if let Some(vt_str) = segs.next().filter(|s| !s.is_empty()) {
                        let idx = vt_str
                            .parse::<usize>()
                            .map_err(|_| format!("Line {}: Invalid UV index", line_num))?;
                        vt_idx =
                            Some(idx.checked_sub(1).ok_or_else(|| {
                                format!("Line {}: UV index 0 is invalid", line_num)
                            })?);
                    }

                    // Look up or insert
                    let key = (v_idx, vt_idx);
                    if let Some(&idx) = unique_vertices.get(&key) {
                        face_indices.push(idx);
                    } else {
                        let new_idx = final_vertices.len();

                        // Push vertex
                        if v_idx >= raw_positions.len() {
                            return Err(format!(
                                "Line {}: Vertex index {} out of bounds",
                                line_num,
                                v_idx + 1
                            ));
                        }
                        // SAFETY: Checked bounds above
                        final_vertices.push(unsafe { *raw_positions.get_unchecked(v_idx) });

                        // Push UV (or default 0,0)
                        if let Some(ti) = vt_idx {
                            if ti >= raw_uvs.len() {
                                return Err(format!(
                                    "Line {}: UV index {} out of bounds",
                                    line_num,
                                    ti + 1
                                ));
                            }
                            // SAFETY: Checked bounds above
                            final_uvs.push(unsafe { *raw_uvs.get_unchecked(ti) });
                        } else {
                            final_uvs.push(Vec2::new(0.0, 0.0));
                        }

                        unique_vertices.insert(key, new_idx);
                        face_indices.push(new_idx);
                    };
                }

                // Triangulate fan
                if face_indices.len() < 3 {
                    return Err(format!("Line {}: Face has fewer than 3 vertices", line_num));
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
