use crate::math::{Vec2, Vec3};
use crate::mesh::Mesh;
use std::collections::HashMap;

/// Load a Mesh from a Wavefront OBJ string source.
///
/// # Errors
/// Returns a string describing the error if parsing fails (e.g. invalid syntax, out of bounds indices).
pub fn load_obj(source: &str) -> Result<Mesh, String> {
    let parser = ObjParser::new();
    parser.parse(source)
}

struct ObjParser {
    raw_positions: Vec<Vec3>,
    raw_uvs: Vec<Vec2>,
    unique_vertices: HashMap<(usize, Option<usize>), usize>,
    final_vertices: Vec<Vec3>,
    final_uvs: Vec<Vec2>,
    final_indices: Vec<[usize; 3]>,
}

impl ObjParser {
    fn new() -> Self {
        Self {
            raw_positions: Vec::with_capacity(1024),
            raw_uvs: Vec::with_capacity(1024),
            unique_vertices: HashMap::with_capacity(1024),
            final_vertices: Vec::with_capacity(1024),
            final_uvs: Vec::with_capacity(1024),
            final_indices: Vec::with_capacity(1024),
        }
    }

    fn parse(mut self, source: &str) -> Result<Mesh, String> {
        let mut face_indices = Vec::with_capacity(4);

        for (line_num, line) in source.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut parts = line.split_whitespace();
            let cmd = parts.next().unwrap_or("");

            match cmd {
                "v" => self.parse_vertex(line_num, parts)?,
                "vt" => self.parse_uv(line_num, parts)?,
                "f" => self.parse_face(line_num, parts, &mut face_indices)?,
                _ => {}
            }
        }

        Ok(Mesh {
            vertices: self.final_vertices,
            indices: self.final_indices,
            uvs: self.final_uvs,
        })
    }

    fn parse_vertex<'a>(
        &mut self,
        line_num: usize,
        mut parts: impl Iterator<Item = &'a str>,
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
        self.raw_positions.push(Vec3::new(x, y, z));
        Ok(())
    }

    fn parse_uv<'a>(
        &mut self,
        line_num: usize,
        mut parts: impl Iterator<Item = &'a str>,
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
        self.raw_uvs.push(Vec2::new(u, v));
        Ok(())
    }

    fn parse_face<'a>(
        &mut self,
        line_num: usize,
        parts: impl Iterator<Item = &'a str>,
        face_indices: &mut Vec<usize>,
    ) -> Result<(), String> {
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
            let key = (v_idx, vt_idx);
            if let Some(&idx) = self.unique_vertices.get(&key) {
                face_indices.push(idx);
            } else {
                let new_idx = self.final_vertices.len();

                // Push vertex
                if v_idx >= self.raw_positions.len() {
                    return Err(format!(
                        "Line {line_num}: Vertex index {} out of bounds",
                        v_idx + 1
                    ));
                }
                // SAFETY: Checked bounds above
                self.final_vertices
                    .push(unsafe { *self.raw_positions.get_unchecked(v_idx) });

                // Push UV (or default 0,0)
                if let Some(ti) = vt_idx {
                    if ti >= self.raw_uvs.len() {
                        return Err(format!(
                            "Line {line_num}: UV index {} out of bounds",
                            ti + 1
                        ));
                    }
                    // SAFETY: Checked bounds above
                    self.final_uvs
                        .push(unsafe { *self.raw_uvs.get_unchecked(ti) });
                } else {
                    self.final_uvs.push(Vec2::new(0.0, 0.0));
                }

                self.unique_vertices.insert(key, new_idx);
                face_indices.push(new_idx);
            }
        }

        // Triangulate fan
        if face_indices.len() < 3 {
            return Err(format!("Line {line_num}: Face has fewer than 3 vertices"));
        }

        for i in 1..face_indices.len() - 1 {
            self.final_indices
                .push([face_indices[0], face_indices[i], face_indices[i + 1]]);
        }
        Ok(())
    }
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
