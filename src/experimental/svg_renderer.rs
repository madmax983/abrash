//! Experimental SVG Renderer
//!
//! Renders a mesh to an SVG string using a simple wireframe approach.

use crate::math::{Mat4, ScreenPoint, project_to_screen};
use crate::mesh::Mesh;
use std::fmt::Write;

/// Renders a mesh to an SVG string.
///
/// # Arguments
///
/// * `mesh` - The mesh to render.
/// * `mvp` - Model-View-Projection matrix.
/// * `width` - Width of the output SVG.
/// * `height` - Height of the output SVG.
#[must_use]
pub fn render_mesh_to_svg(mesh: &Mesh, mvp: Mat4, width: u32, height: u32) -> String {
    let mut svg = String::new();
    // SVG Header
    writeln!(
        &mut svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    )
    .unwrap();

    // Background (transparent by default)

    // Transform all vertices first
    let transformed_vertices: Vec<ScreenPoint> = mesh
        .vertices
        .iter()
        .map(|v| {
            let (v_clip, w) = mvp.transform_point(*v);
            project_to_screen(v_clip, w, width, height)
        })
        .collect();

    // Draw triangles
    for triangle in &mesh.indices {
        let p0 = transformed_vertices[triangle[0]];
        let p1 = transformed_vertices[triangle[1]];
        let p2 = transformed_vertices[triangle[2]];

        // Simple clipping: Skip if any vertex is behind the camera (inv_w < 0 implies w < 0)
        // Note: This is not true clipping, so triangles spanning the near plane will disappear.
        // For a robust renderer, we'd need to clip against the near plane.
        // But for this experimental feature, dropping is acceptable to avoid artifacts.
        if p0.inv_w < 0.0 || p1.inv_w < 0.0 || p2.inv_w < 0.0 {
            continue;
        }

        writeln!(
            &mut svg,
            r#"<path d="M {} {} L {} {} L {} {} Z" fill="none" stroke="black" stroke-width="1" />"#,
            p0.x, p0.y, p1.x, p1.y, p2.x, p2.y
        )
        .unwrap();
    }

    writeln!(&mut svg, "</svg>").unwrap();
    svg
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    #[test]
    fn test_render_triangle() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
        mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
        mesh.indices.push([0, 1, 2]);

        let mvp = Mat4::identity(); // Identity projection means x,y are directly NDC?
        // transform_point(0,0,0) -> (0,0,0,1)
        // project_to_screen(0,0,0, 1, 100, 100) -> x=50, y=50 (center)

        let svg = render_mesh_to_svg(&mesh, mvp, 100, 100);

        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("path"));
        assert!(svg.contains("stroke=\"black\""));
        // Check for coordinates. Center is 50, 50.
        // v0 (0,0,0) -> (0,0,0,1) -> (50, 50)
        // v1 (1,0,0) -> (1,0,0,1) -> (100, 50)
        // v2 (0,1,0) -> (0,1,0,1) -> (50, 0)
        assert!(svg.contains("M 50 50"));
        assert!(svg.contains("L 100 50"));
        assert!(svg.contains("L 50 0"));
    }
}
