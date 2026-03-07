#![cfg(all(feature = "nova", test))]

use std::f32::consts::PI;

use abrash::experimental::modifiers::{displace_noise, taper, twist};
use abrash::mesh::Mesh;

#[test]
fn test_twist_modifier() {
    let mut mesh = Mesh::cube(1.0);
    // Cube has corners at (+/-0.5, +/-0.5, +/-0.5)

    // Twist by PI radians (180 deg) over height of 1.0 (from y=-0.5 to y=0.5)
    twist(&mut mesh, PI);

    // Vertex at y=0 should not be twisted (it's at the pivot of twist)
    // Actually, let's just make sure the top/bottom vertices moved correctly.
    // Bottom vertices (y = -0.5) should be twisted by -PI/2
    // Top vertices (y = 0.5) should be twisted by PI/2

    for v in &mesh.vertices {
        if (v.y - 0.5).abs() < 1e-4 {
            // At top (y = 0.5)
            // if original was (0.5, 0.5, 0.5), twist by PI/2 means x' = x*cos - z*sin = -0.5
            // z' = x*sin + z*cos = 0.5
            // So distance from origin in XZ plane shouldn't change
            let r2 = v.x * v.x + v.z * v.z;
            assert!((r2 - 0.5).abs() < 1e-4);
        } else if (v.y + 0.5).abs() < 1e-4 {
            // At bottom (y = -0.5)
            let r2 = v.x * v.x + v.z * v.z;
            assert!((r2 - 0.5).abs() < 1e-4);
        }
    }
}

#[test]
fn test_taper_modifier() {
    let mut mesh = Mesh::cube(2.0);
    // Cube has corners at (+/-1.0, +/-1.0, +/-1.0)

    // Taper factor 0.5. At y=1.0, scale is 1.0 + 0.5 * 1.0 = 1.5. At y=-1.0, scale is 1.0 + 0.5 * -1.0 = 0.5
    taper(&mut mesh, 0.5);

    for v in &mesh.vertices {
        if (v.y - 1.0).abs() < 1e-4 {
            // Top vertices
            assert!((v.x.abs() - 1.5).abs() < 1e-4);
            assert!((v.z.abs() - 1.5).abs() < 1e-4);
        } else if (v.y + 1.0).abs() < 1e-4 {
            // Bottom vertices
            assert!((v.x.abs() - 0.5).abs() < 1e-4);
            assert!((v.z.abs() - 0.5).abs() < 1e-4);
        }
    }
}

#[test]
fn test_displace_noise_modifier() {
    let mut mesh = Mesh::cube(1.0);
    let _ = mesh.compute_face_normals(); // ensure normals are populated

    // Ensure normals exist
    if mesh.normals.len() != mesh.vertices.len() {
        mesh.normals.resize(mesh.vertices.len(), abrash::math::Vec3::new(0.0, 1.0, 0.0));
    }

    // Displace by 0.5. Since we don't strictly test the noise value here, just make sure vertices moved along normal.
    let original_vertices = mesh.vertices.clone();
    let normals = mesh.normals.clone();

    displace_noise(&mut mesh, 0.5, 12345);

    for i in 0..mesh.vertices.len() {
        let v_orig = original_vertices[i];
        let v_new = mesh.vertices[i];
        let n = normals[i];

        // Ensure displacement happened
        assert!(v_orig != v_new);

        let delta = v_new - v_orig;

        // Since noise returns a value between 0 and 1, delta should be some scalar multiple of normal.
        // It should be proportional to `n`.
        let dot = delta.dot(n);

        // Displaced distance should be strictly positive (noise > 0) or 0 (but rare), and max 0.5
        assert!(dot > 0.0);
        assert!(dot <= 0.5);
    }
}
