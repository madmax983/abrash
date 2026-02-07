use abrash::obj_loader;
use abrash::math::Vec3;

#[test]
fn test_load_obj_integration() {
    let obj_source = "
v 1.0 2.0 3.0
v 4.0 5.0 6.0
v 7.0 8.0 9.0
f 1 2 3
";
    let mesh = obj_loader::load_obj(obj_source).expect("Failed to load OBJ");

    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.indices.len(), 1);
    assert_eq!(mesh.vertices[0], Vec3::new(1.0, 2.0, 3.0));
}
