use abrash::softbody::SoftBody;
use abrash::math::Vec3;
use abrash::mesh::Mesh;

#[test]
fn test_softbody_invalid_indices_crash() {
    // 1. Create a simple valid mesh (single triangle)
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
    mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
    mesh.indices.push([0, 1, 2]);

    // 2. Create SoftBody
    let mut softbody = SoftBody::new(mesh, 1.0, 10.0, 0.5).expect("Failed to create SoftBody");

    // 3. Corrupt the mesh indices
    // The mesh has 3 vertices (indices 0, 1, 2).
    // We add a triangle with index 100, which is out of bounds.
    softbody.mesh.indices.push([0, 1, 100]);

    // 4. Update
    // This calls recompute_normals internally, which iterates indices and accesses vertices.
    // Before the fix, this should panic.
    softbody.update(0.1);
}
