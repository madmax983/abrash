//! SVG Wireframe Exporter.
//!
//! This module provides functionality to export a 3D `Mesh` to a 2D Vector Graphic (SVG) file,
//! preserving the projected geometry as resolution-independent wireframes.

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use std::fmt::Write;

/// A utility to export a 3D `Mesh` to an SVG wireframe document.
pub struct SvgExporter;

impl SvgExporter {
    /// Exports the wireframe of a `Mesh` from the perspective of a camera (`view_proj`) to an SVG `String`.
    ///
    /// # Arguments
    ///
    /// * `mesh` - The 3D mesh to export.
    /// * `view_proj` - The combined View-Projection matrix for the camera.
    /// * `width` - The output SVG document width in pixels.
    /// * `height` - The output SVG document height in pixels.
    /// * `stroke_color` - The SVG stroke color.
    /// * `stroke_width` - The width of the wireframe lines.
    ///
    /// # Returns
    ///
    /// A `String` containing the complete SVG document.
    #[must_use]
    pub fn export_wireframe(
        mesh: &Mesh,
        view_proj: &Mat4,
        width: u32,
        height: u32,
        stroke_color: &str,
        stroke_width: f32,
    ) -> String {
        let mut svg = String::new();

        // 1. Write the SVG header
        let _ = writeln!(
            &mut svg,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\">"
        );

        let half_w = (width as f32) * 0.5;
        let half_h = (height as f32) * 0.5;

        // 2. Iterate through mesh triangles
        for indices in &mesh.indices {
            let v0_local = mesh.vertices[indices[0]];
            let v1_local = mesh.vertices[indices[1]];
            let v2_local = mesh.vertices[indices[2]];

            // 3. Transform to clip space
            let (v0_clip, w0) = view_proj.transform_point(v0_local);
            let (v1_clip, w1) = view_proj.transform_point(v1_local);
            let (v2_clip, w2) = view_proj.transform_point(v2_local);

            // 4. Basic culling: If ANY vertex is behind the near plane (w <= 0), discard the whole triangle.
            if w0 <= 0.0 || w1 <= 0.0 || w2 <= 0.0 {
                continue;
            }

            // 5. Perspective divide (Normalized Device Coordinates: -1.0 to 1.0)
            let v0_ndc = Vec3::new(v0_clip.x / w0, v0_clip.y / w0, v0_clip.z / w0);
            let v1_ndc = Vec3::new(v1_clip.x / w1, v1_clip.y / w1, v1_clip.z / w1);
            let v2_ndc = Vec3::new(v2_clip.x / w2, v2_clip.y / w2, v2_clip.z / w2);

            // 6. Map to screen space (SVG coordinate system: origin at top-left)
            let sx0 = (v0_ndc.x + 1.0) * half_w;
            let sy0 = (1.0 - v0_ndc.y) * half_h;

            let sx1 = (v1_ndc.x + 1.0) * half_w;
            let sy1 = (1.0 - v1_ndc.y) * half_h;

            let sx2 = (v2_ndc.x + 1.0) * half_w;
            let sy2 = (1.0 - v2_ndc.y) * half_h;

            // 7. Write the SVG polygon element
            let _ = writeln!(
                &mut svg,
                "    <polygon points=\"{sx0:.2},{sy0:.2} {sx1:.2},{sy1:.2} {sx2:.2},{sy2:.2}\" fill=\"none\" stroke=\"{stroke_color}\" stroke-width=\"{stroke_width}\" stroke-linejoin=\"round\" />"
            );
        }

        // 8. Close SVG tag
        svg.push_str("</svg>\n");

        svg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::math::{Mat4, Vec3};
    use abrash_core::mesh::Mesh;

    #[test]
    fn test_svg_export_basic_triangle() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(0.0, 1.0, 0.5));
        mesh.vertices.push(Vec3::new(-1.0, -1.0, 0.5));
        mesh.vertices.push(Vec3::new(1.0, -1.0, 0.5));
        mesh.indices.push([0, 1, 2]);

        let view_proj = Mat4::orthographic(-2.0, 2.0, -2.0, 2.0, 0.1, 1.0);

        let svg = SvgExporter::export_wireframe(&mesh, &view_proj, 100, 100, "black", 1.0);

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("fill=\"none\""));
        assert!(svg.contains("stroke=\"black\""));
    }
}
