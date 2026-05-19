#![cfg(feature = "nova")]

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_render::experimental::svg_export::SvgExporter;

#[test]
fn test_svg_export_creates_file_with_expected_content() {
    let mut exporter = SvgExporter::new(800, 600);

    // Create a simple mesh (cube-ish wireframe)
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(-1.0, -1.0, -1.0));
    mesh.vertices.push(Vec3::new(1.0, -1.0, -1.0));
    mesh.vertices.push(Vec3::new(1.0, 1.0, -1.0));
    mesh.vertices.push(Vec3::new(-1.0, 1.0, -1.0));

    mesh.indices.push([0, 1, 2]);
    mesh.indices.push([0, 2, 3]);

    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 3.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.57, 800.0 / 600.0, 0.1, 100.0);
    let view_proj = view * proj;

    exporter.draw_mesh_wireframe(&mesh, &view_proj, "#FF00FF");
    let svg = exporter.build();

    // Basic structural checks
    assert!(svg.starts_with("<svg width=\"800\" height=\"600\""));
    assert!(svg.contains("<rect width=\"100%\" height=\"100%\" fill=\"black\" />"));

    // Check that we have 6 lines (3 per triangle)
    assert_eq!(svg.matches("<line").count(), 6);

    // Check the stroke color
    assert!(svg.contains("stroke=\"#FF00FF\""));
}
