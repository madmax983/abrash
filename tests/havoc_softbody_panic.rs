#![cfg(feature = "nova")]

use abrash::experimental::jelly::SoftBody;
use abrash::mesh::Mesh;
use abrash::math::Vec3;

#[test]
fn test_softbody_panic_repro() {
    // Create a mesh with one vertex
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));

    // Add a triangle that references non-existent vertices (indices 1 and 2)
    mesh.indices.push([0, 1, 2]);

    // This used to panic. Now it should return a Result::Err.
    let result = SoftBody::new(mesh, 1.0, 1.0, 0.5);
    assert!(result.is_err(), "SoftBody::new should return Err for invalid mesh indices");
}
