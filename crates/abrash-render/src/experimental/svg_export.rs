//! SVG Mesh Exporter
//!
//! A simple exporter to convert a 3D `Mesh` into a 2D Scalable Vector Graphics (SVG) file.
//! This is useful for creating wireframe diagrams, technical drawings, or vector art
//! directly from the engine's 3D geometry without external rendering tools.
//!
//! It applies a simple perspective projection to the mesh vertices and outputs standard SVG `<polygon>` and `<line>` tags.

#![cfg(feature = "nova")]

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use std::fmt::Write;

/// Configuration for the SVG export.
#[derive(Debug, Clone)]
pub struct SvgExportConfig {
    /// Width of the output SVG canvas.
    pub width: f32,
    /// Height of the output SVG canvas.
    pub height: f32,
    /// Stroke color for the wireframe lines (e.g., "black", "#FF0000").
    pub stroke_color: String,
    /// Stroke width for the wireframe lines.
    pub stroke_width: f32,
    /// Fill color for polygons. Set to "none" for wireframe.
    pub fill_color: String,
    /// Projection matrix to apply to the mesh before exporting.
    pub view_proj_matrix: Mat4,
}

impl Default for SvgExportConfig {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            stroke_color: "black".to_string(),
            stroke_width: 1.0,
            fill_color: "none".to_string(),
            view_proj_matrix: Mat4::identity(),
        }
    }
}

/// Exports a 3D Mesh to an SVG string.
///
/// It applies the `view_proj_matrix` to transform vertices into clip space,
/// performs perspective division, and maps them to the SVG canvas coordinates.
///
/// Returns `None` if the mesh is empty or string formatting fails.
#[must_use]
pub fn export_mesh_to_svg(mesh: &Mesh, config: &SvgExportConfig) -> Option<String> {
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return None;
    }

    let mut svg = String::new();

    // SVG Header
    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}">"#,
        config.width, config.height, config.width, config.height
    );
    svg.push('\n');

    // Group for styling
    let _ = write!(
        svg,
        r#"  <g stroke="{}" stroke-width="{}" fill="{}">"#,
        config.stroke_color, config.stroke_width, config.fill_color
    );
    svg.push('\n');

    // Half dimensions for NDC to Screen mapping
    let half_w = config.width / 2.0;
    let half_h = config.height / 2.0;

    for &tri in &mesh.indices {
        // Get original vertices
        let v0 = mesh.vertices[tri[0]];
        let v1 = mesh.vertices[tri[1]];
        let v2 = mesh.vertices[tri[2]];

        // Transform vertices
        let (p0_clip, w0) = config.view_proj_matrix.transform_point(v0);
        let (p1_clip, w1) = config.view_proj_matrix.transform_point(v1);
        let (p2_clip, w2) = config.view_proj_matrix.transform_point(v2);

        // Simple clipping (discard if any vertex is fully behind camera)
        if w0 <= 0.0 || w1 <= 0.0 || w2 <= 0.0 {
            continue;
        }

        // Perspective divide
        let inv_w0 = 1.0 / w0;
        let inv_w1 = 1.0 / w1;
        let inv_w2 = 1.0 / w2;

        let ndc0 = Vec3::new(p0_clip.x * inv_w0, p0_clip.y * inv_w0, p0_clip.z * inv_w0);
        let ndc1 = Vec3::new(p1_clip.x * inv_w1, p1_clip.y * inv_w1, p1_clip.z * inv_w1);
        let ndc2 = Vec3::new(p2_clip.x * inv_w2, p2_clip.y * inv_w2, p2_clip.z * inv_w2);

        // Map NDC to Screen (SVG coordinates: origin top-left)
        // NDC x: [-1, 1] -> [0, width]
        // NDC y: [-1, 1] -> [height, 0] (invert Y for SVG)
        let s0_x = (ndc0.x + 1.0) * half_w;
        let s0_y = (1.0 - ndc0.y) * half_h;

        let s1_x = (ndc1.x + 1.0) * half_w;
        let s1_y = (1.0 - ndc1.y) * half_h;

        let s2_x = (ndc2.x + 1.0) * half_w;
        let s2_y = (1.0 - ndc2.y) * half_h;

        // Output Polygon
        let _ = write!(
            svg,
            r#"    <polygon points="{s0_x:.2},{s0_y:.2} {s1_x:.2},{s1_y:.2} {s2_x:.2},{s2_y:.2}" />"#
        );
        svg.push('\n');
    }

    svg.push_str("  </g>\n");
    svg.push_str("</svg>\n");

    Some(svg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_empty_mesh() {
        let mesh = Mesh::new();
        let config = SvgExportConfig::default();
        let svg = export_mesh_to_svg(&mesh, &config);
        assert!(svg.is_none());
    }

    #[test]
    fn test_export_triangle() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(-1.0, -1.0, 5.0));
        mesh.vertices.push(Vec3::new(1.0, -1.0, 5.0));
        mesh.vertices.push(Vec3::new(0.0, 1.0, 5.0));
        mesh.indices.push([0, 1, 2]);

        // Simple orthographic-like projection (identity but mapped directly to NDC)
        // Since w will be 1, NDC = Clip
        let config = SvgExportConfig {
            width: 100.0,
            height: 100.0,
            view_proj_matrix: Mat4::identity(),
            ..Default::default()
        };

        let svg = export_mesh_to_svg(&mesh, &config).unwrap();

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("<polygon points="));

        // (-1, -1) -> NDC (-1, -1) -> Screen (0, 100)
        // (1, -1) -> NDC (1, -1) -> Screen (100, 100)
        // (0, 1) -> NDC (0, 1) -> Screen (50, 0)
        assert!(svg.contains("0.00,100.00 100.00,100.00 50.00,0.00"));
    }
}
