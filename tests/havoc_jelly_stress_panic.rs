#![cfg(feature = "nova")]

use abrash::experimental::jelly::SoftBody;
use abrash::math::Vec3;
use abrash::mesh::Mesh;

#[test]
#[should_panic(expected = "index out of bounds")]
fn test_havoc_jelly_stress_panic() {
    // 1. Setup a valid SoftBody
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
    mesh.indices.push([0, 1, 0]); // dummy triangle

    let mut jelly = SoftBody::new(mesh, 1.0, 1.0, 0.5).unwrap();

    // 2. Sabotage: Truncate the vertices array
    // This makes the existing spring indices invalid.
    jelly.mesh.vertices.clear();

    // 3. Detonate: Call get_vertex_stress()
    // It creates an array of size mesh.vertices.len() (which is 0)
    // and then blindly uses spring indices to access it.
    let _stress = jelly.get_vertex_stress();
}
