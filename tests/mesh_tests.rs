use abrash::mesh::Mesh;

#[test]
fn test_cube_face_normals() {
    let cube = Mesh::cube(2.0);
    let normals = cube.compute_face_normals();

    // 12 triangles = 12 normals
    assert_eq!(normals.len(), 12);

    // All normals should be unit length
    for n in &normals {
        let len = n.length();
        assert!((len - 1.0).abs() < 0.001, "Normal not unit length: {len}");
    }
}
