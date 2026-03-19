#![cfg(feature = "nova")]

use abrash::experimental::jelly::SoftBody;
use abrash::experimental::sdf::{SdfObject, SdfPrimitive, SdfScene};
use abrash::math::Vec3;
use abrash::mesh::Mesh;

#[test]
#[should_panic(expected = "index out of bounds")]
fn test_havoc_softbody_panic() {
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    mesh.indices.push([0, 0, 0]);

    let mut jelly = SoftBody::new(mesh, 1.0, 10.0, 0.5).unwrap();

    let mut scene = SdfScene::new();
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
