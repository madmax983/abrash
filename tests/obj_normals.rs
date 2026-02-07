use abrash::obj_loader::load_obj;
use abrash::math::Vec3;

#[test]
fn test_load_obj_with_normals() {
    let obj_source = "
v 0.0 0.0 0.0
v 1.0 0.0 0.0
v 0.0 1.0 0.0
vn 0.0 0.0 1.0
f 1//1 2//1 3//1
";

    let mesh = load_obj(obj_source).expect("Failed to load OBJ");

    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.normals.len(), 3, "Should have 3 normals, one per vertex");

    // Check normal value
    assert_eq!(mesh.normals[0], Vec3::new(0.0, 0.0, 1.0));
    assert_eq!(mesh.normals[1], Vec3::new(0.0, 0.0, 1.0));
    assert_eq!(mesh.normals[2], Vec3::new(0.0, 0.0, 1.0));
}

#[test]
fn test_load_obj_with_mixed_attributes() {
    // Test format v/vt/vn
    let obj_source = "
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
f 1/1/1 2/2/1 3/3/1
";

    let mesh = load_obj(obj_source).expect("Failed to load OBJ");

    assert_eq!(mesh.vertices.len(), 3);
    assert_eq!(mesh.uvs.len(), 3);
    assert_eq!(mesh.normals.len(), 3);
}
