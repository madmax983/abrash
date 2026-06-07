#![cfg(feature = "nova")]
use abrash::experimental::jelly::SoftBody;
use abrash::math::Vec3;
use abrash::mesh::Mesh;

#[test]
fn test_truncated_mesh_resilience() {
    // 1. Setup a valid SoftBody
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
    mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
    mesh.indices.push([0, 1, 2]);

    let mut jelly = SoftBody::new(mesh, 1.0, 1.0, 0.5).unwrap();

    // 2. Sabotage: Truncate the vertices
    // The springs still refer to indices 0, 1, 2.
    // Truncating to 0 will make index 0 out of bounds.
    jelly.mesh.vertices.clear();

    // 3. Detonate: Call update
    // This should NOT panic. It should handle it gracefully (e.g. by skipping update or logging error).
    jelly.update(0.1);
}

#[test]
fn test_havoc_softbody_panic() {
    use abrash::experimental::sdf::{SdfObject, SdfPrimitive, SdfScene};
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    mesh.indices.push([0, 0, 0]);

    let mut jelly = SoftBody::new(mesh, 1.0, 10.0, 0.5).unwrap();

    let mut scene = SdfScene::with_capacity(1);
    scene.add(SdfObject {
        primitive: SdfPrimitive::Sphere {
            radius: 10.0,
            center: Vec3::new(0.0, 0.0, 0.0),
        },
        color: 0xFFFF_FFFF,
    });

    jelly.velocities.clear();
    jelly.collide_sdf(&scene, 0.5);
}
