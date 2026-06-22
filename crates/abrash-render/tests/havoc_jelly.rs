use abrash_render::experimental::jelly::SoftBody;
use abrash_core::mesh::Mesh;
use abrash_core::math::Vec3;
use proptest::prelude::*;

proptest! {
    #[test]
    #[should_panic]
    fn exploit_jelly_oob(idx in 1000usize..2000usize) {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
        // Provide out of bounds indices in the mesh!
        mesh.indices.push([idx, idx, idx]);

        // SoftBody::new computes springs from mesh indices and will panic!
        let _sb = SoftBody::new(mesh, 1.0, 1.0, 1.0).unwrap();
    }
}
