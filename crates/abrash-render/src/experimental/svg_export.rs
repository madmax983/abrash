//! SVG Vector Exporter
//!
//! Exports 3D meshes to 2D SVG vector graphics.

#![cfg(feature = "nova")]

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;

use std::fmt::Write;

/// Exports a 3D mesh to an SVG string using the given view-projection matrix.
#[must_use]
pub fn export_mesh_to_svg(mesh: &Mesh, view_proj: &Mat4, width: u32, height: u32) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    );
    let _ = writeln!(out, r#"<g stroke="black" stroke-width="1" fill="none">"#);

    let w_f32 = width as f32;
    let h_f32 = height as f32;

    for face in &mesh.indices {
        if face.len() != 3 {
            continue;
        }
        let v0_world = mesh.vertices[face[0]];
        let v1_world = mesh.vertices[face[1]];
        let v2_world = mesh.vertices[face[2]];

        let (p0_clip, _) = view_proj.transform_point(v0_world);
        let (p1_clip, _) = view_proj.transform_point(v1_world);
        let (p2_clip, _) = view_proj.transform_point(v2_world);

        // Map from clip space [-1, 1] to screen space [0, width/height]
        let x0 = (p0_clip.x + 1.0) * 0.5 * w_f32;
        let y0 = (1.0 - p0_clip.y) * 0.5 * h_f32;

        let x1 = (p1_clip.x + 1.0) * 0.5 * w_f32;
        let y1 = (1.0 - p1_clip.y) * 0.5 * h_f32;

        let x2 = (p2_clip.x + 1.0) * 0.5 * w_f32;
        let y2 = (1.0 - p2_clip.y) * 0.5 * h_f32;

        let _ = writeln!(
            out,
            r#"  <polygon points="{x0:.2},{y0:.2} {x1:.2},{y1:.2} {x2:.2},{y2:.2}" />"#
        );
    }

    let _ = writeln!(out, "</g>");
    let _ = writeln!(out, "</svg>");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_export_basic() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(0.0, 0.5, 0.0));
        mesh.vertices.push(Vec3::new(-0.5, -0.5, 0.0));
        mesh.vertices.push(Vec3::new(0.5, -0.5, 0.0));
        mesh.indices.push([0, 1, 2]);

        let view_proj = Mat4::identity();

        let svg = export_mesh_to_svg(&mesh, &view_proj, 800, 600);

        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("<polygon"));
    }
}
