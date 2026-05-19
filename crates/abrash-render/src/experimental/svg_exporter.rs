//! SVG Wireframe Exporter
//!
//! Exports a 3D mesh to a Scalable Vector Graphics (SVG) file as a 2D wireframe
//! using a provided view-projection matrix.

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Exports a mesh as an SVG wireframe.
///
/// # Arguments
///
/// * `mesh` - The mesh to export.
/// * `view_proj` - The view-projection matrix to transform vertices to clip space.
/// * `width` - The output SVG width in pixels.
/// * `height` - The output SVG height in pixels.
/// * `path` - The destination file path.
/// * `stroke_color` - The SVG stroke color (e.g., "#ffffff").
/// * `stroke_width` - The width of the lines in the SVG.
///
/// # Errors
///
/// Returns an error if writing to the file fails.
pub fn export_mesh_to_svg<P: AsRef<Path>>(
    mesh: &Mesh,
    view_proj: &Mat4,
    width: u32,
    height: u32,
    path: P,
    stroke_color: &str,
    stroke_width: f32,
) -> io::Result<()> {
    let mut file = File::create(path)?;

    // Write SVG header
    writeln!(
        file,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#,
    )?;
    writeln!(
        file,
        r#"  <rect width="{width}" height="{height}" fill="black" />"#,
    )?;

    let half_w = width as f32 / 2.0;
    let half_h = height as f32 / 2.0;

    // Transform all vertices and convert to screen space
    let mut screen_vertices = Vec::with_capacity(mesh.vertices.len());
    for v in &mesh.vertices {
        let (clip_v, w) = view_proj.transform_point(*v);

        // Simple near plane clipping
        if w < 0.1 {
            screen_vertices.push(None);
            continue;
        }

        // Perspective divide
        let ndc_x = clip_v.x / w;
        let ndc_y = clip_v.y / w;
        let ndc_z = clip_v.z / w;

        // Viewport transform
        let screen_x = (ndc_x + 1.0) * half_w;
        // Invert Y for SVG coordinates (SVG y=0 is top, NDC y=1 is top)
        let screen_y = (1.0 - ndc_y) * half_h;

        screen_vertices.push(Some(Vec3::new(screen_x, screen_y, ndc_z)));
    }

    // Process triangles
    for tri in &mesh.indices {
        let i0 = tri[0];
        let i1 = tri[1];
        let i2 = tri[2];

        if let (Some(v0), Some(v1), Some(v2)) = (
            screen_vertices[i0],
            screen_vertices[i1],
            screen_vertices[i2],
        ) {
            // Simple backface culling in screen space (2D cross product)
            // (v1.x - v0.x) * (v2.y - v0.y) - (v1.y - v0.y) * (v2.x - v0.x)
            // Note: SVG Y is inverted relative to NDC, so the winding order check is reversed.
            let cross = (v1.x - v0.x) * (v2.y - v0.y) - (v1.y - v0.y) * (v2.x - v0.x);

            // Assuming counter-clockwise winding is front-facing in object space.
            // After inverted Y projection, front-facing triangles have positive cross product.
            if cross <= 0.0 {
                continue; // Backfacing
            }

            writeln!(
                file,
                r#"  <polygon points="{},{} {},{} {},{}" fill="none" stroke="{}" stroke-width="{}" stroke-linejoin="round" />"#,
                v0.x, v0.y, v1.x, v1.y, v2.x, v2.y, stroke_color, stroke_width
            )?;
        }
    }

    writeln!(file, "</svg>")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::math::Vec3;
    use std::fs;

    #[test]
    fn test_export_mesh_to_svg_red_phase() {
        let mesh = Mesh::cube(2.0);

        let view = Mat4::look_at(
            Vec3::new(3.0, 3.0, 3.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
        let view_proj = view * proj;

        let path = "test_export_wireframe.svg";

        let res = export_mesh_to_svg(&mesh, &view_proj, 800, 800, path, "#00ff00", 1.5);

        assert!(res.is_ok());

        let svg_content = fs::read_to_string(path).unwrap();
        assert!(svg_content.contains("<svg"));
        assert!(svg_content.contains("</svg>"));
        assert!(svg_content.contains("<polygon"));

        // Clean up
        let _ = fs::remove_file(path);
    }
}
