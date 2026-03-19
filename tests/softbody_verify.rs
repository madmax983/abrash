#![cfg(feature = "nova")]

use abrash::experimental::jelly::SoftBody;
use abrash::math::Vec3;
use abrash::mesh::Mesh;

fn create_simple_mesh() -> Mesh {
    let mut mesh = Mesh::new();
    // 2 vertices connected by an edge
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0)); // Dist 1.0
    // Triangle to satisfy mesh structure (needs 3 indices)
    mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
    mesh.indices.push([0, 1, 2]);
    mesh
}

#[test]
fn test_softbody_physics_step() {
    let mesh = create_simple_mesh();
    let mut softbody = SoftBody::new(mesh, 1.0, 10.0, 0.1).unwrap();

    // Initial state
    let y0 = softbody.mesh.vertices[0].y;

    // Run one update
    softbody.update(0.1);

    let y1 = softbody.mesh.vertices[0].y;

    // Gravity should pull it down
    assert!(y1 < y0, "Vertex 0 should fall: {y1} < {y0}");

    // Check forces are reset
    for f in &softbody.forces {
        assert_eq!(
            f.length().to_bits(),
            0.0f32.to_bits(),
            "Forces should be reset after update"
        );
    }
}

#[test]
fn test_softbody_spring_force() {
    let mesh = create_simple_mesh();
    let mut softbody = SoftBody::new(mesh, 1.0, 100.0, 0.0).unwrap();

    // Stretch the spring manually by moving vertex 1
    softbody.mesh.vertices[1] = Vec3::new(2.0, 0.0, 0.0); // Rest length is 1.0

    // Zero velocities to isolate spring force
    softbody.velocities[0] = Vec3::ZERO;
    softbody.velocities[1] = Vec3::ZERO;

    // Run update
    softbody.update(0.01);

    let v0 = softbody.velocities[0];
    let v1 = softbody.velocities[1];

    // Vertex 0 should be pulled towards Vertex 1 (positive X)
    assert!(v0.x > 0.0, "Vertex 0 should be pulled right: {v0:?}");
    // Vertex 1 should be pulled towards Vertex 0 (negative X)
    assert!(v1.x < 0.0, "Vertex 1 should be pulled left: {v1:?}");
}
