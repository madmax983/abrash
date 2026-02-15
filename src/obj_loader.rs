//! Wavefront OBJ file loader.
//!
//! This module provides a simple parser for `.obj` files.
//!
//! # Supported Features
//!
//! *   **Vertices (`v`)**: 3D positions (x, y, z).
//! *   **Texture Coordinates (`vt`)**: 2D UVs (u, v).
//! *   **Normals (`vn`)**: 3D normal vectors (nx, ny, nz).
//! *   **Faces (`f`)**: Triangles and Quads (automatically triangulated).
//!     *   Supports `v`, `v/vt`, `v//vn`, and `v/vt/vn` formats.
//!
//! # Limitations
//!
//! *   **Materials (`usemtl`, `mtllib`)**: Ignored.
//! *   **Groups (`g`, `o`)**: Ignored.
//!
//! # Performance
//!
//! This loader implements several optimizations for high-performance parsing:
//!
//! *   **Vertex Deduplication**: Uses a `HashMap` to reuse vertices with identical attributes.
//! *   **Fast Parsing**: Uses [`split_ascii_whitespace`](str::split_ascii_whitespace) to avoid Unicode
//!     property lookups, which provides a ~20% speedup for ASCII files.
//! *   **Integer Parsing**: Uses a custom `fast_parse_usize` function to parse indices without
//!     standard library overhead.

use crate::math::{Vec2, Vec3};
use crate::mesh::Mesh;
use std::collections::HashMap;
use std::str::SplitAsciiWhitespace;

/// Optimized integer parser for OBJ indices.
/// Replaces generic `str::parse::<usize>` to avoid overhead.
#[inline]
fn fast_parse_usize(bytes: &[u8]) -> Option<usize> {
    if bytes.is_empty() || bytes.len() > 20 {
        return None;
    }
    let mut n: usize = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add((b - b'0') as usize)?;
    }
    Some(n)
}

struct ObjLoader {
    raw_positions: Vec<Vec3>,
    raw_uvs: Vec<Vec2>,
    raw_normals: Vec<Vec3>,
    deduplicator: HashMap<u64, usize>,
    final_vertices: Vec<Vec3>,
    final_uvs: Vec<Vec2>,
    final_normals: Vec<Vec3>,
    final_indices: Vec<[usize; 3]>,
}

impl ObjLoader {
    fn new() -> Self {
        Self {
            raw_positions: Vec::with_capacity(1024),
            raw_uvs: Vec::with_capacity(1024),
            raw_normals: Vec::with_capacity(1024),
            deduplicator: HashMap::with_capacity(1024),
            final_vertices: Vec::with_capacity(1024),
            final_uvs: Vec::with_capacity(1024),
            final_normals: Vec::with_capacity(1024),
            final_indices: Vec::with_capacity(1024),
        }
    }

    fn parse_vertex(
        &mut self,
        parts: &mut SplitAsciiWhitespace,
        line_num: usize,
    ) -> Result<(), String> {
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
        if self.raw_positions.len() >= 1_000_000 {
            return Err(format!("Line {line_num}: Maximum vertices exceeded"));
        }
        self.raw_positions.push(Vec3::new(x, y, z));
        Ok(())
    }

    fn parse_uv(
        &mut self,
        parts: &mut SplitAsciiWhitespace,
        line_num: usize,
    ) -> Result<(), String> {
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
        if self.raw_uvs.len() >= 1_000_000 {
            return Err(format!("Line {line_num}: Maximum UVs exceeded"));
        }
        self.raw_uvs.push(Vec2::new(u, v));
        Ok(())
    }

    fn parse_normal(
        &mut self,
        parts: &mut SplitAsciiWhitespace,
        line_num: usize,
    ) -> Result<(), String> {
        let x = parts
            .next()
            .ok_or_else(|| format!("Line {line_num}: Missing nx"))?
            .parse::<f32>()
            .map_err(|_| format!("Line {line_num}: Invalid nx"))?;
        let y = parts
            .next()
            .ok_or_else(|| format!("Line {line_num}: Missing ny"))?
            .parse::<f32>()
            .map_err(|_| format!("Line {line_num}: Invalid ny"))?;
        let z = parts
            .next()
            .ok_or_else(|| format!("Line {line_num}: Missing nz"))?
            .parse::<f32>()
            .map_err(|_| format!("Line {line_num}: Invalid nz"))?;

        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(format!(
                "Line {line_num}: Normal coordinates must be finite"
            ));
        }
        if self.raw_normals.len() >= 1_000_000 {
            return Err(format!("Line {line_num}: Maximum Normals exceeded"));
        }
        self.raw_normals.push(Vec3::new(x, y, z).normalize());
        Ok(())
    }

    fn parse_face(&mut self, parts: SplitAsciiWhitespace, line_num: usize) -> Result<(), String> {
        let mut face_indices = Vec::with_capacity(4);

        for part in parts {
            let bytes = part.as_bytes();

            // Find first '/'
            let mut first_slash = bytes.len();
            for (i, &b) in bytes.iter().enumerate() {
                if b == b'/' {
                    first_slash = i;
                    break;
                }
            }

            // Parse v_idx
            let v_idx = fast_parse_usize(&bytes[0..first_slash])
                .ok_or_else(|| format!("Line {line_num}: Invalid vertex index"))?;
            let v_idx = v_idx
                .checked_sub(1)
                .ok_or_else(|| format!("Line {line_num}: Vertex index 0 is invalid"))?;

            let mut vt_idx = None;
            let mut vn_idx = None;

            if first_slash < bytes.len() {
                let after_first_slash = first_slash + 1;
                if after_first_slash < bytes.len() && bytes[after_first_slash] == b'/' {
                    // v//vn
                    let after_second_slash = after_first_slash + 1;
                    if after_second_slash < bytes.len() {
                        let idx = fast_parse_usize(&bytes[after_second_slash..])
                            .ok_or_else(|| format!("Line {line_num}: Invalid Normal index"))?;
                        vn_idx = Some(idx.checked_sub(1).ok_or_else(|| {
                            format!("Line {line_num}: Normal index 0 is invalid")
                        })?);
                    }
                } else if after_first_slash < bytes.len() {
                    // v/vt...
                    let mut end_vt = bytes.len();
                    let mut second_slash = None;

                    for (i, &b) in bytes.iter().enumerate().skip(after_first_slash) {
                        if b == b'/' {
                            end_vt = i;
                            second_slash = Some(i);
                            break;
                        }
                    }

                    let vt_bytes = &bytes[after_first_slash..end_vt];
                    if !vt_bytes.is_empty() {
                        let idx = fast_parse_usize(vt_bytes)
                            .ok_or_else(|| format!("Line {line_num}: Invalid UV index"))?;
                        vt_idx =
                            Some(idx.checked_sub(1).ok_or_else(|| {
                                format!("Line {line_num}: UV index 0 is invalid")
                            })?);
                    }

                    if let Some(slash2) = second_slash {
                        let after_second_slash = slash2 + 1;
                        if after_second_slash < bytes.len() {
                            let idx = fast_parse_usize(&bytes[after_second_slash..])
                                .ok_or_else(|| format!("Line {line_num}: Invalid Normal index"))?;
                            vn_idx = Some(idx.checked_sub(1).ok_or_else(|| {
                                format!("Line {line_num}: Normal index 0 is invalid")
                            })?);
                        }
                    }
                }
            }

            // Validate indices
            if v_idx >= self.raw_positions.len() {
                return Err(format!(
                    "Line {line_num}: Vertex index {} out of bounds",
                    v_idx + 1
                ));
            }

            // Deduplicate
            const SENTINEL: u64 = 0xF_FFFF;
            let k_v = v_idx as u64;
            let k_vt = vt_idx.map(|i| i as u64).unwrap_or(SENTINEL);
            let k_vn = vn_idx.map(|i| i as u64).unwrap_or(SENTINEL);
            let key = k_v | (k_vt << 20) | (k_vn << 40);

            let final_idx = if let Some(&idx) = self.deduplicator.get(&key) {
                idx
            } else {
                let new_idx = self.final_vertices.len();
                self.final_vertices.push(self.raw_positions[v_idx]);

                if let Some(ti) = vt_idx {
                    if ti >= self.raw_uvs.len() {
                        return Err(format!(
                            "Line {line_num}: UV index {} out of bounds",
                            ti + 1
                        ));
                    }
                    self.final_uvs.push(self.raw_uvs[ti]);
                } else {
                    self.final_uvs.push(Vec2::new(0.0, 0.0));
                }

                if let Some(ni) = vn_idx {
                    if ni >= self.raw_normals.len() {
                        return Err(format!(
                            "Line {line_num}: Normal index {} out of bounds",
                            ni + 1
                        ));
                    }
                    self.final_normals.push(self.raw_normals[ni]);
                } else {
                    self.final_normals.push(Vec3::new(0.0, 0.0, 0.0));
                }

                self.deduplicator.insert(key, new_idx);
                new_idx
            };

            face_indices.push(final_idx);
        }

        // Triangulate
        if face_indices.len() < 3 {
            return Err(format!("Line {line_num}: Face has fewer than 3 vertices"));
        }

        for i in 1..face_indices.len() - 1 {
            if self.final_indices.len() >= 1_000_000 {
                return Err(format!("Line {line_num}: Maximum faces exceeded"));
            }
            self.final_indices
                .push([face_indices[0], face_indices[i], face_indices[i + 1]]);
        }
        Ok(())
    }
}

/// Load a Mesh from a Wavefront OBJ string source.
///
/// This function parses vertices, UVs, normals, and faces.
/// It automatically triangulates quads and deduplicates vertices.
///
/// # Supported Tags
/// * `v` - Vertex position (x, y, z)
/// * `vt` - Texture coordinate (u, v)
/// * `vn` - Vertex normal (x, y, z)
/// * `f` - Face indices (v/vt/vn)
///
/// # Examples
///
/// ```
/// use abrash::obj_loader::load_obj;
///
/// let obj_source = "
/// v -1.0 -1.0 0.0
/// v  1.0 -1.0 0.0
/// v  0.0  1.0 0.0
/// f 1 2 3
/// ";
///
/// let mesh = load_obj(obj_source).expect("Failed to parse OBJ");
///
/// // Iterate over triangles
/// for (i, triangle_indices) in mesh.indices.iter().enumerate() {
///     let v0 = mesh.vertices[triangle_indices[0]];
///     let v1 = mesh.vertices[triangle_indices[1]];
///     let v2 = mesh.vertices[triangle_indices[2]];
///
///     println!("Triangle {}: {:?}, {:?}, {:?}", i, v0, v1, v2);
/// }
///
/// assert_eq!(mesh.vertices.len(), 3);
/// assert_eq!(mesh.indices.len(), 1);
/// ```
#[allow(clippy::missing_errors_doc)]
pub fn load_obj(source: &str) -> Result<Mesh, String> {
    let mut loader = ObjLoader::new();

    for (line_num, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_ascii_whitespace();
        let cmd = parts.next().unwrap_or("");

        match cmd {
            "v" => loader.parse_vertex(&mut parts, line_num)?,
            "vt" => loader.parse_uv(&mut parts, line_num)?,
            "vn" => loader.parse_normal(&mut parts, line_num)?,
            "f" => loader.parse_face(parts, line_num)?,
            _ => {}
        }
    }

    // Post-processing
    if loader.raw_normals.is_empty() {
        loader.final_normals.clear();
    }

    Ok(Mesh {
        vertices: loader.final_vertices,
        indices: loader.final_indices,
        uvs: loader.final_uvs,
        normals: loader.final_normals,
        tangents: Vec::new(),
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

    #[test]
    fn test_load_normals() {
        let obj = "
v 0 0 0
v 1 0 0
v 0 1 0
vn 0 1 0
f 1//1 2//1 3//1
";
        let mesh = load_obj(obj).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.normals.len(), 3);
        assert_eq!(mesh.normals[0], Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn test_load_mixed_normals() {
        let obj = "
v 0 0 0
v 1 0 0
v 0 1 0
vn 0 1 0
f 1//1 2 3
";
        let mesh = load_obj(obj).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.normals.len(), 3);
        assert_eq!(mesh.normals[0], Vec3::new(0.0, 1.0, 0.0));
        // Second vertex has no normal specified, should default to zero
        assert_eq!(mesh.normals[1], Vec3::new(0.0, 0.0, 0.0));
    }
}
