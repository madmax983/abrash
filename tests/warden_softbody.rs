#[cfg(feature = "nova")]
#[test]
fn test_softbody_invalid_indices() {
    use abrash::experimental::jelly::SoftBody;
    use abrash::mesh::Mesh;
    use abrash::math::Vec3;

    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    // Index 1 is out of bounds (len is 1)
    mesh.indices.push([0, 1, 0]);

    // This call should return Err.
    // Before the fix, this function returns SoftBody and panics internally.
    // After the fix, it returns Result<SoftBody, String>.
    // Since we are changing the API, this test is written for the new API.
    let result = SoftBody::new(mesh, 1.0, 1.0, 0.5);

    assert!(result.is_err(), "SoftBody::new should return Err for invalid indices");
}
