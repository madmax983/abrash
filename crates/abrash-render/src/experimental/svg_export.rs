use abrash_core::math::{Mat4, Vec2, Vec3};
use abrash_core::mesh::Mesh;
use std::fmt::Write;

/// A lightweight, dependency-free SVG Exporter for 2D and 3D wireframe graphics.
pub struct SvgExporter {
    width: u32,
    height: u32,
    lines: Vec<(f32, f32, f32, f32, String)>,
}

impl SvgExporter {
    /// Creates a new SVG Exporter with the specified document dimensions.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            lines: Vec::new(),
        }
    }

    /// Adds a 2D line to the SVG document.
    pub fn add_line(&mut self, start: Vec2, end: Vec2, color: &str) {
        self.lines.push((start.x, start.y, end.x, end.y, color.to_string()));
    }

    /// Projects a 3D mesh into 2D using the provided view-projection matrix and adds its wireframe to the SVG.
    pub fn draw_mesh_wireframe(&mut self, mesh: &Mesh, view_proj: &Mat4, color: &str) {
        let mut projected_verts = Vec::with_capacity(mesh.vertices.len());
        let hw = self.width as f32 / 2.0;
        let hh = self.height as f32 / 2.0;

        for &v in &mesh.vertices {
            let (clip_v, w) = view_proj.transform_point(v);

            // Basic near-plane clipping
            if w <= 0.0 {
                projected_verts.push(None);
                continue;
            }

            // Perspective divide
            let ndc_x = clip_v.x / w;
            let ndc_y = clip_v.y / w;

            // Screen space mapping (Y is flipped in SVG)
            let screen_x = (ndc_x + 1.0) * hw;
            let screen_y = (1.0 - ndc_y) * hh;

            projected_verts.push(Some(Vec2::new(screen_x, screen_y)));
        }

        for idx in &mesh.indices {
            let i0 = idx[0];
            let i1 = idx[1];
            let i2 = idx[2];

            if let (Some(p0), Some(p1), Some(p2)) = (
                projected_verts[i0],
                projected_verts[i1],
                projected_verts[i2],
            ) {
                self.add_line(p0, p1, color);
                self.add_line(p1, p2, color);
                self.add_line(p2, p0, color);
            }
        }
    }

    /// Builds and returns the final SVG XML string.
    #[must_use]
    pub fn build(&self) -> String {
        let mut svg = String::new();
        let _ = write!(svg, "<svg width=\"{}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\">\n", self.width, self.height);

        // Add a background rect for visibility in dark viewers
        svg.push_str("  <rect width=\"100%\" height=\"100%\" fill=\"black\" />\n");

        for (x1, y1, x2, y2, color) in &self.lines {
            let _ = write!(svg, "  <line x1=\"{x1:.2}\" y1=\"{y1:.2}\" x2=\"{x2:.2}\" y2=\"{y2:.2}\" stroke=\"{color}\" stroke-width=\"1\" />\n");
        }

        svg.push_str("</svg>\n");
        svg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_export_empty() {
        let exporter = SvgExporter::new(800, 600);
        let svg = exporter.build();
        assert!(svg.contains("<svg width=\"800\" height=\"600\""));
        assert!(svg.contains("</svg>"));
        // Only the rect background should be there, no lines
        assert!(!svg.contains("<line"));
    }

    #[test]
    fn test_svg_export_lines() {
        let mut exporter = SvgExporter::new(800, 600);
        exporter.add_line(Vec2::new(10.0, 10.0), Vec2::new(20.0, 20.0), "red");
        let svg = exporter.build();
        assert!(svg.contains("<line x1=\"10.00\" y1=\"10.00\" x2=\"20.00\" y2=\"20.00\" stroke=\"red\" stroke-width=\"1\" />"));
    }

    #[test]
    fn test_svg_export_mesh() {
        let mut exporter = SvgExporter::new(100, 100);
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
        mesh.vertices.push(Vec3::new(-1.0, -1.0, 0.0));
        mesh.vertices.push(Vec3::new(1.0, -1.0, 0.0));
        mesh.indices.push([0, 1, 2]);

        // Identity matrix for view-proj
        let view_proj = Mat4::identity();

        exporter.draw_mesh_wireframe(&mesh, &view_proj, "white");
        let svg = exporter.build();

        // Should contain 3 lines for the triangle
        assert_eq!(svg.matches("<line").count(), 3);
    }
}
