use abrash::mesh::Mesh;

#[test]
fn test_cube_vertex_normals() {
    let cube = Mesh::cube(2.0);
    let normals = cube.compute_vertex_normals();

    // 8 vertices = 8 normals
    assert_eq!(normals.len(), 8);

    // All normals should be unit length
    for n in &normals {
        let len = n.length();
        assert!((len - 1.0).abs() < 0.001, "Normal not unit length: {}", len);
    }

    // Corner vertex normal should point diagonally outward
    // Vertex 0 is at (-h, -h, h), normal should point roughly (-1, -1, 1) normalized
    let n0 = normals[0];
    assert!(n0.x < 0.0);
    assert!(n0.y < 0.0);
    assert!(n0.z > 0.0);
}

#[test]
fn test_cube_face_normals() {
    let cube = Mesh::cube(2.0);
    let normals = cube.compute_face_normals();

    // 12 triangles = 12 normals
    assert_eq!(normals.len(), 12);

    // All normals should be unit length
    for n in &normals {
        let len = n.length();
        assert!((len - 1.0).abs() < 0.001, "Normal not unit length: {}", len);
    }
}
