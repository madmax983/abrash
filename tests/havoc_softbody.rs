use abrash::experimental::jelly::SoftBody;
use abrash::mesh::Mesh;
use abrash::math::Vec3;

#[test]
fn test_havoc_softbody_indices_oob_check() {
    // 1. Create a mesh with 1 vertex
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));

    // 2. But reference index 999
    mesh.indices.push([0, 999, 0]);

    // 3. Create SoftBody - This should now fail gracefully
    let jelly_result = SoftBody::new(mesh, 1.0, 1.0, 1.0);

    assert!(jelly_result.is_err(), "SoftBody::new should return Err for invalid indices");

    match jelly_result {
        Ok(_) => panic!("SoftBody::new should have failed"),
        Err(err) => assert!(err.contains("Index 999"), "Error message should mention the invalid index"),
    }
}

#[test]
fn test_havoc_softbody_mutation_resilience() {
    // Test that mutating mesh AFTER creation doesn't crash update/recompute_normals
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
    mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
    mesh.indices.push([0, 1, 2]);

    // Create valid SoftBody
    let mut jelly = SoftBody::new(mesh, 1.0, 1.0, 1.0).expect("Valid mesh should pass");

    // MUTATE indices to be invalid
    // 999 is definitely out of bounds
    jelly.mesh.indices[0] = [0, 999, 2];

    // Call update - Should NOT panic
    jelly.update(0.1);
}
