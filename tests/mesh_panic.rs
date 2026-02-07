use abrash::mesh::Mesh;
use abrash::math::Vec3;

#[test]
fn test_compute_face_normals_panic() {
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    mesh.indices.push([0, 1, 2]); // Index 1 and 2 are out of bounds

    let result = mesh.compute_face_normals();
    assert!(result.is_err());
    assert_eq!(result.err(), Some("Vertex index out of bounds"));
}

#[test]
fn test_compute_face_normals_valid() {
    let mut mesh = Mesh::new();
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
    mesh.vertices.push(Vec3::new(0.0, 1.0, 0.0));
    mesh.indices.push([0, 1, 2]);

    let result = mesh.compute_face_normals();
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_degenerate_triangle_normals() {
    let mut mesh = Mesh::new();
    // Collinear vertices (line, not triangle)
    mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
    mesh.vertices.push(Vec3::new(1.0, 0.0, 0.0));
    mesh.vertices.push(Vec3::new(2.0, 0.0, 0.0));
    mesh.indices.push([0, 1, 2]);

    let result = mesh.compute_face_normals();
    assert!(result.is_ok());
    let normals = result.unwrap();
    let n = normals[0];

    // Should be zero vector because cross product is zero
    assert_eq!(n.x, 0.0);
    assert_eq!(n.y, 0.0);
    assert_eq!(n.z, 0.0);

    // Also check for NaNs
    assert!(!n.x.is_nan());
    assert!(!n.y.is_nan());
    assert!(!n.z.is_nan());
}
