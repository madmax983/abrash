use abrash::experimental::volume::Volume;
use abrash::math::Vec3;
use abrash::mesh::Mesh;

#[test]
fn test_volume_creation_and_modification() {
    let mut volume = Volume::new(10, 10, 10, 1.0, Vec3::new(0.0, 0.0, 0.0));

    // Should be empty initially
    let mesh = volume.to_mesh_optimized();
    assert!(mesh.vertices.is_empty());

    // Deposit a single voxel at (5,5,5)
    // Radius 0.6 to cover the center voxel
    volume.deposit(Vec3::new(5.5, 5.5, 5.5), 0.6);

    // Should now have 1 voxel set
    assert!(volume.grid.get(5, 5, 5));

    let mesh = volume.to_mesh_optimized();
    // A single cube has 6 faces * 2 tris = 12 triangles.
    assert_eq!(mesh.indices.len(), 12);
    // 6 faces * 4 verts = 24 vertices (no sharing in my implementation yet)
    assert_eq!(mesh.vertices.len(), 24);

    // Carve it out
    volume.carve(Vec3::new(5.5, 5.5, 5.5), 0.6);
    assert!(!volume.grid.get(5, 5, 5));

    let mesh = volume.to_mesh_optimized();
    assert!(mesh.vertices.is_empty());
}

#[test]
fn test_mesh_optimization() {
    let mut volume = Volume::new(5, 5, 5, 1.0, Vec3::new(0.0, 0.0, 0.0));

    // Create a 3x3x3 block in the center
    // Center of grid is 2.5, 2.5, 2.5.
    // 3x3x3 block spans from 1.0 to 4.0.

    for z in 1..4 {
        for y in 1..4 {
            for x in 1..4 {
                volume.grid.set(x, y, z, true);
            }
        }
    }

    // Naive mesh count (if we used naive implementation)
    // 27 voxels * 12 triangles = 324 triangles.

    // Optimized mesh count (hidden face removal)
    // Surface area of 3x3x3 cube is 6 faces * (3*3) = 54 quads.
    // 54 * 2 = 108 triangles.

    let mesh = volume.to_mesh_optimized();
    assert_eq!(mesh.indices.len(), 108);

    // Verify by carving the center voxel
    // (2,2,2) is the center of 1..4 range (indices 1,2,3)
    volume.grid.set(2, 2, 2, false);

    // Now we exposed the inner faces.
    // Removed 1 voxel (which was hidden).
    // Exposed 6 internal faces of neighbors.
    // Original outer surface: 54 quads.
    // Plus 6 new inner quads.
    // Total 60 quads = 120 triangles.

    let mesh = volume.to_mesh_optimized();
    assert_eq!(mesh.indices.len(), 120);
}

#[test]
fn test_from_mesh_integration() {
    // Create a simple mesh (cube)
    let cube = Mesh::cube(2.0); // -1 to 1

    // Voxelize with resolution 4
    let volume = Volume::from_mesh(&cube, 4);

    // Should have some voxels
    let mesh = volume.to_mesh_optimized();
    assert!(!mesh.vertices.is_empty());
}
