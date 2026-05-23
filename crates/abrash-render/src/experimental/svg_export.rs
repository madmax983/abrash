//! SVG Exporter
//!
//! An experimental feature that exports a 3D mesh directly to a 2D Vector Graphics (SVG) string.
//! Instead of rasterizing pixels, this module projects the 3D vertices into screen space,
//! sorts the triangles back-to-front (Painter's Algorithm), and outputs formatted `<svg>` tags.
//!
//! This provides infinite-resolution "wireframe" or flat-shaded exports of 3D geometry.

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;

/// Projects a 3D mesh and exports it as an SVG string.
///
/// * `mesh` - The 3D geometry to export.
/// * `view_proj` - The combined View * Projection matrix.
/// * `width` - Output SVG width.
/// * `height` - Output SVG height.
/// * `wireframe` - If true, draws unfilled polygons with strokes. If false, fills them with flat shading.
pub fn export_mesh_to_svg(
    mesh: &Mesh,
    view_proj: &Mat4,
    width: u32,
    height: u32,
    wireframe: bool,
) -> String {
    let mut projected_triangles = Vec::new();

    let half_w = (width as f32) / 2.0;
    let half_h = (height as f32) / 2.0;

    // Project all vertices and assemble triangles
    for indices in &mesh.indices {
        let i0 = indices[0];
        let i1 = indices[1];
        let i2 = indices[2];

        // Ensure indices are valid
        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
            continue;
        }

        let v0 = mesh.vertices[i0];
        let v1 = mesh.vertices[i1];
        let v2 = mesh.vertices[i2];

        // Transform to clip space
        let (clip0, w0) = view_proj.transform_point(v0);
        let (clip1, w1) = view_proj.transform_point(v1);
        let (clip2, w2) = view_proj.transform_point(v2);

        // Simple frustum culling (if any vertex is behind camera)
        if w0 <= 0.0 || w1 <= 0.0 || w2 <= 0.0 {
            continue;
        }

        // Perspective divide
        let ndc0 = Vec3::new(clip0.x / w0, clip0.y / w0, clip0.z / w0);
        let ndc1 = Vec3::new(clip1.x / w1, clip1.y / w1, clip1.z / w1);
        let ndc2 = Vec3::new(clip2.x / w2, clip2.y / w2, clip2.z / w2);

        // Calculate face normal (in NDC space for flat shading)
        let edge1 = ndc1 - ndc0;
        let edge2 = ndc2 - ndc0;
        let normal = Vec3::cross(edge1, edge2);

        // Backface culling
        if normal.z >= 0.0 {
            continue;
        }

        // To Screen Space
        let scr0 = Vec3::new(ndc0.x * half_w + half_w, -ndc0.y * half_h + half_h, ndc0.z);
        let scr1 = Vec3::new(ndc1.x * half_w + half_w, -ndc1.y * half_h + half_h, ndc1.z);
        let scr2 = Vec3::new(ndc2.x * half_w + half_w, -ndc2.y * half_h + half_h, ndc2.z);

        let avg_z = (scr0.z + scr1.z + scr2.z) / 3.0;

        // Basic flat shading color calculation (simulating light from -Z)
        let intensity = (-normal.z).max(0.1).min(1.0);
        let base_color = 200.0;
        let c = (base_color * intensity) as u8;
        let color = format!("#{:02x}{:02x}{:02x}", c, c, c);

        projected_triangles.push(ProjectedTriangle {
            v0: scr0,
            v1: scr1,
            v2: scr2,
            z: avg_z,
            color,
        });
    }

    // Painter's Algorithm: Sort back to front (largest Z to smallest Z in NDC)
    projected_triangles.sort_by(|a, b| b.z.partial_cmp(&a.z).unwrap_or(std::cmp::Ordering::Equal));

    // Construct SVG
    let mut svg = String::with_capacity(projected_triangles.len() * 128 + 128);
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">\n",
        width, height, width, height
    ));
    // Optional background
    svg.push_str(&format!(
        "  <rect width=\"100%\" height=\"100%\" fill=\"#1a1a1a\"/>\n"
    ));

    for tri in projected_triangles {
        let style = if wireframe {
            "fill=\"none\" stroke=\"#00ff00\" stroke-width=\"1.0\""
        } else {
            // Need a slight stroke of the same color to avoid anti-aliasing gaps between polygons
            &format!("fill=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"", tri.color, tri.color)
        };

        svg.push_str(&format!(
            "  <polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" {}/>\n",
            tri.v0.x, tri.v0.y,
            tri.v1.x, tri.v1.y,
            tri.v2.x, tri.v2.y,
            style
        ));
    }

    svg.push_str("</svg>");
    svg
}

struct ProjectedTriangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    z: f32,
    color: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::math::{Mat4, Vec2, Vec3};
    use abrash_core::mesh::Mesh;

    #[test]
    fn test_export_mesh_to_svg() {
        let mut mesh = Mesh::new();
        // A single triangle centered at the origin
        mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
        mesh.vertices.push(Vec3::new(-1.0, -1.0, 0.0));
        mesh.vertices.push(Vec3::new(1.0, -1.0, 0.0));
        mesh.indices.push([0, 1, 2]);

        // Place camera back a bit looking at origin
        let eye = Vec3::new(0.0, 0.0, 5.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);
        let proj = Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let view_proj = view * proj;

        let width = 800;
        let height = 800;

        let svg = export_mesh_to_svg(&mesh, &view_proj, width, height, true);
        assert!(svg.contains("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"800\" height=\"800\" viewBox=\"0 0 800 800\">"));
        assert!(svg.contains("<polygon points="));
        assert!(svg.contains("</svg>"));

        // Since it's a wireframe, it should contain stroke attributes
        assert!(svg.contains("fill=\"none\" stroke=\"#00ff00\" stroke-width=\"1.0\""));
    }

    #[test]
    fn test_export_mesh_to_svg_flat_shaded() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
        mesh.vertices.push(Vec3::new(-1.0, -1.0, 0.0));
        mesh.vertices.push(Vec3::new(1.0, -1.0, 0.0));
        mesh.indices.push([0, 1, 2]);

        let eye = Vec3::new(0.0, 0.0, 5.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);
        let proj = Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let view_proj = view * proj;

        let svg = export_mesh_to_svg(&mesh, &view_proj, 800, 800, false);
        assert!(svg.contains("<polygon points="));
        assert!(!svg.contains("fill=\"none\"")); // Should have a solid fill
    }
}
