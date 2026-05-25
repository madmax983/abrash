use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use std::fmt::Write;

pub struct SvgExporter {
    pub width: u32,
    pub height: u32,
    pub background_color: Option<&'static str>,
    pub stroke_color: &'static str,
    pub stroke_width: f32,
}

impl Default for SvgExporter {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            background_color: Some("#1e1e2e"),
            stroke_color: "#a6e3a1",
            stroke_width: 1.0,
        }
    }
}

impl SvgExporter {
    #[must_use]
    pub fn export_mesh(&self, mesh: &Mesh, view_proj: &Mat4) -> String {
        let mut svg = String::new();
        let _ = write!(
            svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}">"#,
            self.width, self.height, self.width, self.height
        );

        if let Some(bg) = self.background_color {
            let _ = write!(
                svg,
                "\n  <rect width=\"100%\" height=\"100%\" fill=\"{bg}\"/>\n"
            );
        } else {
            let _ = write!(svg, "\n");
        }

        // Project vertices
        let mut projected = Vec::with_capacity(mesh.vertices.len());
        for v in &mesh.vertices {
            let (clip_pos, w) = view_proj.transform_point(*v);
            let ndc = if w == 0.0 {
                clip_pos
            } else {
                Vec3::new(clip_pos.x / w, clip_pos.y / w, clip_pos.z / w)
            };

            let screen_x = (ndc.x + 1.0) * 0.5 * (self.width as f32);
            let screen_y = (1.0 - ndc.y) * 0.5 * (self.height as f32);
            projected.push(Vec3::new(screen_x, screen_y, ndc.z));
        }

        let _ = write!(
            svg,
            "  <g stroke=\"{}\" stroke-width=\"{:.1}\" fill=\"none\" stroke-linejoin=\"round\">\n",
            self.stroke_color, self.stroke_width
        );

        for indices in &mesh.indices {
            let p0 = projected[indices[0]];
            let p1 = projected[indices[1]];
            let p2 = projected[indices[2]];

            // Simple backface culling (cross product of 2D screen coordinates)
            let v1x = p1.x - p0.x;
            let v1y = p1.y - p0.y;
            let v2x = p2.x - p0.x;
            let v2y = p2.y - p0.y;
            let cross = (v1x * v2y) - (v1y * v2x);

            // In screen space with Y down, positive cross product means front-facing
            // depending on winding order. Here we assume standard CCW.
            // Adjust threshold or polarity if needed.
            if cross > 0.0 {
                let _ = write!(
                    svg,
                    "    <polygon points=\"{:.2},{:.2} {:.2},{:.2} {:.2},{:.2}\"/>\n",
                    p0.x, p0.y, p1.x, p1.y, p2.x, p2.y
                );
            }
        }

        let _ = write!(svg, "  </g>\n</svg>");
        svg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_export_failing() {
        let mesh = Mesh::cube(2.0);
        let view_proj = Mat4::identity();
        let exporter = SvgExporter::default();

        let svg = exporter.export_mesh(&mesh, &view_proj);
        assert!(svg.contains("<svg"), "Should output valid SVG wrapper");
        assert!(svg.contains("<polygon"), "Should output polygons");
    }
}
