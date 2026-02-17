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

const MAX_VERTICES: usize = 1_000_000;
const MAX_FACES: usize = 1_000_000;
// 0xFFFFF is used as a sentinel for NO_INDEX.
const SENTINEL: u64 = 0xF_FFFF;

struct LoaderContext {
    raw_positions: Vec<Vec3>,
    raw_uvs: Vec<Vec2>,
    raw_normals: Vec<Vec3>,
    deduplicator: HashMap<u64, usize>,
    final_vertices: Vec<Vec3>,
    final_uvs: Vec<Vec2>,
    final_normals: Vec<Vec3>,
    final_indices: Vec<[usize; 3]>,
    face_indices: Vec<usize>,
}

impl LoaderContext {
    fn new(estimated_capacity: usize) -> Self {
        Self {
            raw_positions: Vec::with_capacity(estimated_capacity),
            raw_uvs: Vec::with_capacity(estimated_capacity),
            raw_normals: Vec::with_capacity(estimated_capacity),
            deduplicator: HashMap::with_capacity(estimated_capacity),
            final_vertices: Vec::with_capacity(estimated_capacity),
            final_uvs: Vec::with_capacity(estimated_capacity),
            final_normals: Vec::with_capacity(estimated_capacity),
            final_indices: Vec::with_capacity(estimated_capacity),
            face_indices: Vec::with_capacity(4),
        }
    }

    fn parse_vertex(
        &mut self,
        line_num: usize,
        parts: &mut std::str::SplitAsciiWhitespace,
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
        if self.raw_positions.len() >= MAX_VERTICES {
            return Err(format!("Line {line_num}: Maximum vertices exceeded"));
        }
        self.raw_positions.push(Vec3::new(x, y, z));
        Ok(())
    }

    fn parse_uv(
        &mut self,
        line_num: usize,
        parts: &mut std::str::SplitAsciiWhitespace,
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
        if self.raw_uvs.len() >= MAX_VERTICES {
            return Err(format!("Line {line_num}: Maximum UVs exceeded"));
        }
        self.raw_uvs.push(Vec2::new(u, v));
        Ok(())
    }

    fn parse_normal(
        &mut self,
        line_num: usize,
        parts: &mut std::str::SplitAsciiWhitespace,
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
        if self.raw_normals.len() >= MAX_VERTICES {
            return Err(format!("Line {line_num}: Maximum Normals exceeded"));
        }
        self.raw_normals.push(Vec3::new(x, y, z).normalize());
        Ok(())
    }

    fn parse_face(
        &mut self,
        line_num: usize,
        parts: std::str::SplitAsciiWhitespace,
    ) -> Result<(), String> {
        self.face_indices.clear();
        for part in parts {
            // format: v, v/vt, v//vn, v/vt/vn
            // Manual parsing to avoid split() iterator overhead
            let bytes = part.as_bytes();

            // Find first '/' to separate v from vt/vn
            // This is faster than split('/').next()
            let mut first_slash = bytes.len();
            for (i, &b) in bytes.iter().enumerate() {
                if b == b'/' {
                    first_slash = i;
                    break;
                }
            }

            // Parse v_idx (0..first_slash)
            let v_idx = fast_parse_usize(&bytes[0..first_slash])
                .ok_or_else(|| format!("Line {line_num}: Invalid vertex index"))?;

            let v_idx = v_idx
                .checked_sub(1)
                .ok_or_else(|| format!("Line {line_num}: Vertex index 0 is invalid"))?;

            // Parse vt_idx and vn_idx if present
            let mut vt_idx = None;
            let mut vn_idx = None;

            if first_slash < bytes.len() {
                let after_first_slash = first_slash + 1;
                // Check if next char is also '/' (case v//vn)
                if after_first_slash < bytes.len() && bytes[after_first_slash] == b'/' {
                    // v//vn case
                    let after_second_slash = after_first_slash + 1;
                    if after_second_slash < bytes.len() {
                        // Parse vn
                        let idx = fast_parse_usize(&bytes[after_second_slash..]).ok_or_else(
                            || format!("Line {line_num}: Invalid Normal index"),
                        )?;
                        vn_idx = Some(idx.checked_sub(1).ok_or_else(|| {
                            format!("Line {line_num}: Normal index 0 is invalid")
                        })?);
                    }
                } else if after_first_slash < bytes.len() {
                    // It's v/vt...
                    // Find end of vt (next slash or end of string)
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
                        let idx = fast_parse_usize(vt_bytes).ok_or_else(|| {
                            format!("Line {line_num}: Invalid UV index")
                        })?;
                        vt_idx = Some(idx.checked_sub(1).ok_or_else(|| {
                            format!("Line {line_num}: UV index 0 is invalid")
                        })?);
                    }

                    // If there's a second slash, parse vn (v/vt/vn)
                    if let Some(slash2) = second_slash {
                        let after_second_slash = slash2 + 1;
                        if after_second_slash < bytes.len() {
                            let idx = fast_parse_usize(&bytes[after_second_slash..])
                                .ok_or_else(|| {
                                    format!("Line {line_num}: Invalid Normal index")
                                })?;
                            vn_idx = Some(idx.checked_sub(1).ok_or_else(|| {
                                format!("Line {line_num}: Normal index 0 is invalid")
                            })?);
                        }
                    }
                }
            }

            // Look up or insert
            if v_idx >= self.raw_positions.len() {
                return Err(format!(
                    "Line {}: Vertex index {} out of bounds",
                    line_num,
                    v_idx + 1
                ));
            }

            // Use HashMap for full deduplication
            // Pack keys into u64 to reduce hashing overhead and memory usage (8 bytes vs 24 bytes)
            // Max index is 1,000,000, which fits in 20 bits (1,048,576).

            let k_v = v_idx as u64;
            let k_vt = vt_idx.map_or(SENTINEL, |i| i as u64);
            let k_vn = vn_idx.map_or(SENTINEL, |i| i as u64);

            let key = k_v | (k_vt << 20) | (k_vn << 40);

            match self.deduplicator.entry(key) {
                std::collections::hash_map::Entry::Occupied(entry) => {
                    self.face_indices.push(*entry.get());
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    let new_idx = self.final_vertices.len();

                    // Push vertex
                    self.final_vertices.push(self.raw_positions[v_idx]);

                    // Push UV (or default 0,0)
                    if let Some(ti) = vt_idx {
                        if ti >= self.raw_uvs.len() {
                            return Err(format!(
                                "Line {}: UV index {} out of bounds",
                                line_num,
                                ti + 1
                            ));
                        }
                        self.final_uvs.push(self.raw_uvs[ti]);
                    } else {
                        self.final_uvs.push(Vec2::new(0.0, 0.0));
                    }

                    // Push Normal (if present)
                    if let Some(ni) = vn_idx {
                        if ni >= self.raw_normals.len() {
                            return Err(format!(
                                "Line {}: Normal index {} out of bounds",
                                line_num,
                                ni + 1
                            ));
                        }
                        self.final_normals.push(self.raw_normals[ni]);
                    } else {
                        // For simplicity: If vn_idx is None, push Zero.
                        self.final_normals.push(Vec3::new(0.0, 0.0, 0.0));
                    }

                    entry.insert(new_idx);
                    self.face_indices.push(new_idx);
                }
            }
        }

        // Triangulate fan
        if self.face_indices.len() < 3 {
            return Err(format!("Line {line_num}: Face has fewer than 3 vertices"));
        }

        for i in 1..self.face_indices.len() - 1 {
            if self.final_indices.len() >= MAX_FACES {
                return Err(format!("Line {line_num}: Maximum faces exceeded"));
            }
            self.final_indices.push([
                self.face_indices[0],
                self.face_indices[i],
                self.face_indices[i + 1],
            ]);
        }
        Ok(())
    }

    fn build_mesh(mut self) -> Mesh {
        // Post-processing: If no normals were parsed, clear the final_normals vector to avoid partial state
        if self.raw_normals.is_empty() {
            self.final_normals.clear();
        }

        Mesh {
            vertices: self.final_vertices,
            indices: self.final_indices,
            uvs: self.final_uvs,
            normals: self.final_normals,
            tangents: Vec::new(), // Tangents must be computed explicitly
        }
    }
}

/// Optimized integer parser for OBJ indices.
/// Replaces generic `str::parse::<usize>` to avoid overhead.
#[inline]
fn fast_parse_usize(bytes: &[u8]) -> Option<usize> {
    // On 64-bit, usize is u64 (max ~1.8e19, 19 full digits).
    // On 32-bit, usize is u32 (max ~4e9, 9 full digits).
    // We strictly reject numbers longer than this to guarantee no overflow without checking.
    const MAX_DIGITS: usize = if std::mem::size_of::<usize>() >= 8 { 19 } else { 9 };

    if bytes.is_empty() {
        return None;
    }

    if bytes.len() > MAX_DIGITS {
        return None;
    }

    let mut n: usize = 0;
    for &b in bytes {
        let d = b.wrapping_sub(b'0');
        if d > 9 {
            return None;
        }
        // No checked_mul/add needed because of MAX_DIGITS check
        n = n * 10 + (d as usize);
    }
    Some(n)
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
    // Reserve reasonable initial capacity to avoid frequent reallocations
    // Heuristic: Estimate count based on file size.
    // Average line length ~40 bytes. Conservative estimate.
    let estimated_capacity = std::cmp::max(1024, source.len() / 40);

    let mut context = LoaderContext::new(estimated_capacity);

    for (line_num, line) in source.lines().enumerate() {
        // Optimization: Use split_ascii_whitespace directly to avoid redundant trim().
        // It handles leading/trailing whitespace automatically.
        let mut parts = line.split_ascii_whitespace();
        let cmd = match parts.next() {
            Some(s) if s.starts_with('#') => continue,
            Some(s) => s,
            None => continue,
        };

        match cmd {
            "v" => context.parse_vertex(line_num, &mut parts)?,
            "vt" => context.parse_uv(line_num, &mut parts)?,
            "vn" => context.parse_normal(line_num, &mut parts)?,
            "f" => context.parse_face(line_num, parts)?,
            _ => {} // Ignore groups (g), materials (usemtl), etc.
        }
    }

    Ok(context.build_mesh())
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

    #[test]
    fn test_empty_input() {
        let mesh = load_obj("").unwrap();
        assert!(mesh.vertices.is_empty());
        assert!(mesh.indices.is_empty());
    }

    #[test]
    fn test_comments_only() {
        let obj = "
# This is a comment
# Another comment
";
        let mesh = load_obj(obj).unwrap();
        assert!(mesh.vertices.is_empty());
    }

    #[test]
    fn test_malformed_lines() {
        // Missing coordinates
        assert!(load_obj("v").is_err());
        assert!(load_obj("v 1.0").is_err());

        // Invalid numbers
        assert!(load_obj("v a b c").is_err());
        assert!(load_obj("v 1.0 2.0 c").is_err());

        // Malformed face
        assert!(load_obj("f").is_err());
        assert!(load_obj("f 1 2").is_err()); // Not a triangle
    }

    #[test]
    fn test_invalid_indices() {
        let obj_ok = "v 0 0 0\nv 1 0 0\nv 0 1 0\n";

        // Index 0 (OBJ is 1-based)
        let obj_zero = format!("{}f 0 1 2", obj_ok);
        assert!(load_obj(&obj_zero).is_err());

        // Index out of bounds
        let obj_oob = format!("{}f 1 2 4", obj_ok); // 4 doesn't exist
        assert!(load_obj(&obj_oob).is_err());
    }

    #[test]
    fn test_finite_checks() {
        // Infinity
        assert!(load_obj("v inf 0 0").is_err());
        // NaN
        assert!(load_obj("v NaN 0 0").is_err());

        // UVs
        assert!(load_obj("vt inf 0").is_err());
    }

    #[test]
    fn test_face_format_parsing() {
        // v//vn format checks
        // 3 vertices, 1 normal
        let obj = "
v 0 0 0
v 1 0 0
v 0 1 0
vn 0 1 0
f 1//1 2//1 3//1
";
        let mesh = load_obj(obj).unwrap();
        assert_eq!(mesh.indices.len(), 1);

        // v/vt/vn format checks
        // 3 vertices, 1 uv, 1 normal
        let obj2 = "
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vn 0 1 0
f 1/1/1 2/1/1 3/1/1
";
        let mesh2 = load_obj(obj2).unwrap();
        assert_eq!(mesh2.indices.len(), 1);
    }
}
