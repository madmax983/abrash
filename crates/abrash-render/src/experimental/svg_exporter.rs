//! SVG Wireframe Exporter
//!
//! An experimental feature to export 3D geometry into 2D Scalable Vector Graphics (SVG) wireframes.
//! It applies a model-view-projection matrix to the mesh, performs perspective divide,
//! and maps coordinates to a 2D viewport, writing raw XML tags manually.

use abrash_core::math::Mat4;
use abrash_core::mesh::Mesh;
use std::fmt::Write;

/// Exports a 3D mesh to a 2D SVG string containing polygon outlines.
#[must_use]
pub fn export_mesh_to_svg(
    mesh: &Mesh,
    mvp: &Mat4,
    viewport_width: f32,
    viewport_height: f32,
    stroke_color: &str,
    stroke_width: f32,
) -> String {
    let mut out = String::new();
    let hw = viewport_width / 2.0;
    let hh = viewport_height / 2.0;

    let _ = write!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {viewport_width} {viewport_height}\">\n"
    );
    out.push_str("  <rect width=\"100%\" height=\"100%\" fill=\"black\" />\n");

    for face in &mesh.indices {
        let mut pts = String::new();
        let mut visible = true;
        for &idx in face {
            let v = mesh.vertices[idx];
            let (clip, w) = mvp.transform_point(v);

            // Very basic culling for things fully behind camera (w <= 0)
            if w <= 0.0001 {
                visible = false;
                break;
            }

            let ndc_x = clip.x / w;
            let ndc_y = clip.y / w;

            let screen_x = (ndc_x + 1.0) * hw;
            let screen_y = (1.0 - ndc_y) * hh; // Flip Y for SVG

            let _ = write!(pts, "{screen_x},{screen_y} ");
        }

        if visible {
            let pts_trimmed = pts.trim_end();
            let _ = write!(
                out,
                "  <polygon points=\"{pts_trimmed}\" fill=\"none\" stroke=\"{stroke_color}\" stroke-width=\"{stroke_width}\" />\n"
            );
        }
    }

    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::math::Vec3;

    #[test]
    fn test_export_mesh_to_svg() {
        let mut mesh = Mesh::new();
        // A simple triangle in clip space
        mesh.vertices.push(Vec3::new(0.0, 0.5, 0.5));
        mesh.vertices.push(Vec3::new(0.5, -0.5, 0.5));
        mesh.vertices.push(Vec3::new(-0.5, -0.5, 0.5));
        mesh.indices.push([0, 1, 2]);

        let mvp = Mat4::identity();
        let svg = export_mesh_to_svg(&mesh, &mvp, 800.0, 600.0, "red", 2.0);

        assert!(svg.contains("<svg"), "Should contain SVG start tag");
        assert!(svg.contains("</svg>"), "Should contain SVG end tag");
        assert!(svg.contains("<polygon"), "Should contain polygon tag");
        assert!(svg.contains("red"), "Should contain stroke color");
        assert!(
            svg.contains("stroke-width=\"2\""),
            "Should contain stroke width"
        );
    }
}
