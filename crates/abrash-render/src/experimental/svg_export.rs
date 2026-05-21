#![cfg(feature = "nova")]

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use std::fmt::Write;

/// Exports a 3D `Mesh` to a 2D SVG wireframe string.
///
/// This experimental function projects the 3D vertices of a given `Mesh` using the
/// provided `view_proj` matrix and formats the resulting 2D triangles into a standard
/// SVG image string. It acts as a bridge between the 3D pipeline and 2D vector graphics.
#[must_use]
pub fn export_mesh_wireframe_to_svg(
    mesh: &Mesh,
    view_proj: &Mat4,
    width: u32,
    height: u32,
    stroke_color: &str,
    stroke_width: f32,
) -> String {
    let mut svg = String::new();
    let width_f = width as f32;
    let height_f = height as f32;

    // Start SVG document
    let _ = writeln!(
        &mut svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width_f} {height_f}" width="{width_f}" height="{height_f}">"#
    );

    // Common styling for polygons
    let style = format!("fill=\"none\" stroke=\"{stroke_color}\" stroke-width=\"{stroke_width}\"");

    // We need at least one triangle to render anything
    if mesh.indices.is_empty() || mesh.vertices.is_empty() {
        let _ = writeln!(&mut svg, "</svg>");
        return svg;
    }

    // Transform and project all vertices
    // 1. Clip space coordinates
    let mut clip_space_verts = Vec::with_capacity(mesh.vertices.len());
    for &v in &mesh.vertices {
        let (clip, w) = view_proj.transform_point(v);
        clip_space_verts.push((clip, w));
    }

    // Iterate over triangles and convert to SVG polygons
    for &[i0, i1, i2] in &mesh.indices {
        // Retrieve clip space coords
        let (c0, w0) = clip_space_verts[i0];
        let (c1, w1) = clip_space_verts[i1];
        let (c2, w2) = clip_space_verts[i2];

        // Basic near-plane clipping (if any vertex is behind the camera, skip the whole triangle)
        if w0 <= 0.0 || w1 <= 0.0 || w2 <= 0.0 {
            continue;
        }

        // Perspective divide to NDC (-1 to 1)
        let ndc0 = c0 / w0;
        let ndc1 = c1 / w1;
        let ndc2 = c2 / w2;

        // Viewport transform to screen space
        let p0x = (ndc0.x + 1.0) * 0.5 * width_f;
        let p0y = (1.0 - ndc0.y) * 0.5 * height_f; // Y-down in SVG

        let p1x = (ndc1.x + 1.0) * 0.5 * width_f;
        let p1y = (1.0 - ndc1.y) * 0.5 * height_f;

        let p2x = (ndc2.x + 1.0) * 0.5 * width_f;
        let p2y = (1.0 - ndc2.y) * 0.5 * height_f;

        // Format the SVG polygon
        let _ = writeln!(
            &mut svg,
            "  <polygon points=\"{p0x},{p0y} {p1x},{p1y} {p2x},{p2y}\" {style} />"
        );
    }

    // End SVG document
    let _ = writeln!(&mut svg, "</svg>");

    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_empty_mesh() {
        let mesh = Mesh::new();
        let view_proj = Mat4::identity();
        let svg = export_mesh_wireframe_to_svg(&mesh, &view_proj, 800, 600, "black", 1.0);

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(!svg.contains("<polygon"));
    }

    #[test]
    fn test_export_simple_triangle() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(0.0, 0.5, 0.0));
        mesh.vertices.push(Vec3::new(-0.5, -0.5, 0.0));
        mesh.vertices.push(Vec3::new(0.5, -0.5, 0.0));
        mesh.indices.push([0, 1, 2]);

        // Use identity matrix, meaning vertices are effectively already in NDC
        // except w will be 1.0 (from Mat4::identity transform)
        let view_proj = Mat4::identity();

        let svg = export_mesh_wireframe_to_svg(&mesh, &view_proj, 100, 100, "red", 2.0);

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("<polygon points="));
        assert!(svg.contains("stroke=\"red\""));
        assert!(svg.contains("stroke-width=\"2\""));

        // Check the transformed coordinates.
        // v0: (0.0, 0.5) -> (0.0+1.0)*0.5*100 = 50.0, (1.0-0.5)*0.5*100 = 25.0
        // v1: (-0.5, -0.5) -> (-0.5+1.0)*0.5*100 = 25.0, (1.0 - -0.5)*0.5*100 = 75.0
        // v2: (0.5, -0.5) -> (0.5+1.0)*0.5*100 = 75.0, (1.0 - -0.5)*0.5*100 = 75.0
        assert!(svg.contains("50,25"));
        assert!(svg.contains("25,75"));
        assert!(svg.contains("75,75"));
    }

    #[test]
    fn test_export_clips_behind_camera() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(0.0, 0.0, -2.0)); // w will be -2 with identity w-component handling (actually wait, Mat4 identity keeps w=1.0)
        // Let's create a custom matrix where w becomes negative
        let mut m = Mat4::identity();
        m.m[3][3] = -1.0; // Force w to be negative

        mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        mesh.indices.push([0, 1, 2]);

        let svg = export_mesh_wireframe_to_svg(&mesh, &m, 100, 100, "red", 1.0);

        // Triangle should be clipped
        assert!(!svg.contains("<polygon"));
    }
}
