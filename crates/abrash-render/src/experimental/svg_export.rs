//! SVG Wireframe Exporter
//!
//! Exports 3D meshes as 2D vector graphics (SVG) by transforming vertices
//! through a camera matrix and drawing wireframe lines.

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use std::fs::File;
use std::io::{Result, Write};

/// An experimental SVG wireframe exporter.
pub struct SvgExporter;

impl SvgExporter {
    /// Exports a mesh as a wireframe SVG.
    ///
    /// # Arguments
    /// * `mesh` - The 3D mesh to export
    /// * `view_proj` - The combined View-Projection matrix
    /// * `width` - Output SVG width in pixels
    /// * `height` - Output SVG height in pixels
    /// * `path` - The file path to save the SVG
    ///
    /// # Errors
    ///
    /// Returns an IO error if the file cannot be created or written to.
    pub fn export_wireframe(
        mesh: &Mesh,
        view_proj: &Mat4,
        width: u32,
        height: u32,
        path: &str,
    ) -> Result<()> {
        let mut file = File::create(path)?;

        writeln!(
            file,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}" style="background-color: black;">"#
        )?;

        // Pre-transform vertices to screen space
        let mut screen_verts = Vec::with_capacity(mesh.vertices.len());
        for &v in &mesh.vertices {
            let (clip_pos, w) = view_proj.transform_point(v);

            // Perspective divide
            let ndc_x = clip_pos.x / w;
            let ndc_y = clip_pos.y / w;

            // Map to screen coordinates (SVG Y is down)
            let screen_x = (ndc_x + 1.0) * 0.5 * (width as f32);
            let screen_y = (1.0 - ndc_y) * 0.5 * (height as f32);

            screen_verts.push((screen_x, screen_y, w));
        }

        // Draw triangles
        for tri in &mesh.indices {
            let v0 = screen_verts[tri[0]];
            let v1 = screen_verts[tri[1]];
            let v2 = screen_verts[tri[2]];

            // Simple clipping: if any vertex is behind the camera (w <= 0.1), skip the triangle
            if v0.2 > 0.1 && v1.2 > 0.1 && v2.2 > 0.1 {
                writeln!(
                    file,
                    r#"  <polygon points="{x0},{y0} {x1},{y1} {x2},{y2}" fill="none" stroke="lime" stroke-width="1" opacity="0.8"/>"#,
                    x0 = v0.0,
                    y0 = v0.1,
                    x1 = v1.0,
                    y1 = v1.1,
                    x2 = v2.0,
                    y2 = v2.1
                )?;
            }
        }

        writeln!(file, "</svg>")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_svg_export() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(0.0, 0.5, 0.0));
        mesh.vertices.push(Vec3::new(-0.5, -0.5, 0.0));
        mesh.vertices.push(Vec3::new(0.5, -0.5, 0.0));
        mesh.indices.push([0, 1, 2]);

        let eye = Vec3::new(0.0, 0.0, 5.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);
        let proj = Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0);
        let view_proj = view * proj;

        let path = "test_triangle.svg";
        let result = SvgExporter::export_wireframe(&mesh, &view_proj, 800, 600, path);
        assert!(result.is_ok());

        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("<svg"));
        assert!(content.contains("<polygon"));
        assert!(content.contains("stroke=\"lime\""));

        // Cleanup
        let _ = fs::remove_file(path);
    }
}
