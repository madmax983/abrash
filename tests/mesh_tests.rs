use abrash::mesh::Mesh;
use abrash::math::Vec3;

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

#[test]
fn test_load_obj_simple() {
    let obj_content = r#"
# Simple Pyramid
v 0.0 1.0 0.0
v -1.0 -1.0 1.0
v 1.0 -1.0 1.0
v 1.0 -1.0 -1.0
v -1.0 -1.0 -1.0

# Base
f 2 3 4 5
# Sides
f 1 2 3
f 1 3 4
f 1 4 5
f 1 5 2
"#;

    let mesh = Mesh::from_obj(obj_content).expect("Failed to parse OBJ");

    // Check vertices (5 vertices)
    assert_eq!(mesh.vertices.len(), 5);
    // Check indices
    // Base is a quad (2 triangles) + 4 sides (4 triangles) = 6 triangles
    assert_eq!(mesh.indices.len(), 6);

    // Verify first vertex
    assert_eq!(mesh.vertices[0], Vec3::new(0.0, 1.0, 0.0));
}

#[test]
fn test_sphere_generation() {
    let radius = 1.0;
    let sectors = 20;
    let stacks = 10;

    let mesh = Mesh::sphere(radius, sectors, stacks);

    // Check we have vertices
    // We expect at least (sectors) * (stacks) vertices, likely more due to seam duplication or poles
    assert!(mesh.vertices.len() >= sectors * stacks);

    // Check we have indices
    // We expect roughly 2 * sectors * stacks triangles
    assert!(mesh.indices.len() >= sectors * stacks);

    // Check bounds roughly
    for v in &mesh.vertices {
        // Allow some float error, but should be on surface
        let len = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
        assert!((len - radius).abs() < 0.01, "Vertex {:?} not on sphere surface", v);
    }
}
