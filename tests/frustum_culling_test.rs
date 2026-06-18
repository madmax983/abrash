use abrash_core::geometry::Frustum;
use abrash::math::{Mat4, Vec3};
use abrash::mesh::Mesh;

#[test]
fn test_frustum_culling_inside() {
    // Create a simple mesh (cube) centered at origin
    let mesh = Mesh::cube(2.0);

    // Calculate bounding sphere
    let sphere = mesh.calculate_bounding_sphere();

    // Create a view-projection matrix looking at the origin from z=5
    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
    let vp = view * proj; // Row-major: view * proj is correct for v * M?
    // Wait, row-major multiplication order for v * M is v' = v * View * Proj.
    // So M = View * Proj.
    // Yes.

    // Extract frustum
    let frustum = Frustum::from_view_projection(&vp);

    // Check visibility
    assert!(
        frustum.intersects_sphere(sphere.center, sphere.radius),
        "Mesh at origin should be visible"
    );
}

#[test]
fn test_frustum_culling_outside() {
    // Create a mesh far away
    let mut mesh = Mesh::cube(2.0);
    // Move vertices far away to (100, 0, 0)
    for v in &mut mesh.vertices {
        *v = *v + Vec3::new(100.0, 0.0, 0.0);
    }

    let sphere = mesh.calculate_bounding_sphere();

    let view = Mat4::look_at(
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);
    let vp = view * proj;

    let frustum = Frustum::from_view_projection(&vp);

    assert!(
        !frustum.intersects_sphere(sphere.center, sphere.radius),
        "Mesh at (100,0,0) should be culled"
    );
}
