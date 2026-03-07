use crate::experimental::procedural_mesh::noise;
use crate::mesh::Mesh;

/// Twists the mesh around the Y axis based on the vertex Y coordinate.
///
/// # Arguments
///
/// * `mesh` - The mesh to twist.
/// * `total_angle` - The total angle to twist from top to bottom (y = -0.5 to y = 0.5).
#[cfg(feature = "parallel")]
use rayon::prelude::*;

pub fn twist(mesh: &mut Mesh, total_angle: f32) {
    #[cfg(feature = "parallel")]
    let iter = mesh.vertices.par_iter_mut();
    #[cfg(not(feature = "parallel"))]
    let iter = mesh.vertices.iter_mut();

    iter.for_each(|v| {
        // Assume twist is applied from y = -0.5 to y = 0.5 for a normalized mesh
        // We'll just map y directly to an angle: angle = y * total_angle
        let angle = v.y * total_angle;
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let new_x = v.x * cos_a - v.z * sin_a;
        let new_z = v.x * sin_a + v.z * cos_a;

        v.x = new_x;
        v.z = new_z;
    });

    // Recalculate normals
    let _ = mesh.compute_face_normals();
    mesh.compute_tangents();
}

/// Tapers the mesh uniformly along the X and Z axes based on the vertex Y coordinate.
///
/// # Arguments
///
/// * `mesh` - The mesh to taper.
/// * `taper_factor` - The amount to taper. A factor of 0.0 means no change.
pub fn taper(mesh: &mut Mesh, taper_factor: f32) {
    #[cfg(feature = "parallel")]
    let iter = mesh.vertices.par_iter_mut();
    #[cfg(not(feature = "parallel"))]
    let iter = mesh.vertices.iter_mut();

    iter.for_each(|v| {
        // Taper based on height (Y). At Y=0, scale is 1.0. At Y=1.0, scale is 1.0 + taper_factor
        let scale = 1.0 + v.y * taper_factor;
        v.x *= scale;
        v.z *= scale;
    });

    // Recalculate normals
    let _ = mesh.compute_face_normals();
    mesh.compute_tangents();
}

/// Displaces the mesh vertices along their normals based on a noise function.
///
/// # Arguments
///
/// * `mesh` - The mesh to displace.
/// * `amount` - The maximum displacement amount.
/// * `seed` - The random seed for the noise function.
pub fn displace_noise(mesh: &mut Mesh, amount: f32, seed: u32) {
    // We need normals to displace along them
    if mesh.normals.len() != mesh.vertices.len() {
        let _ = mesh.compute_face_normals();
    }

    // Fallback if still empty or length mismatch: initialize with UP
    let mut normals = mesh.normals.clone();
    if normals.len() != mesh.vertices.len() {
        normals.resize(mesh.vertices.len(), crate::math::Vec3::new(0.0, 1.0, 0.0));
    }

    #[cfg(feature = "parallel")]
    let iter = mesh.vertices.par_iter_mut().enumerate();
    #[cfg(not(feature = "parallel"))]
    let iter = mesh.vertices.iter_mut().enumerate();

    iter.for_each(|(i, v)| {
        // Use the procedural noise function from procedural_mesh
        // We evaluate noise based on the X and Z coordinates
        let n = noise(v.x, v.z, seed);

        let normal = normals[i];

        // Displace the vertex along its normal
        *v = *v + normal * (n * amount);
    });

    // Recalculate normals after displacement
    let _ = mesh.compute_face_normals();
    mesh.compute_tangents();
}
