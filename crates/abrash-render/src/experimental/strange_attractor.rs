use abrash_core::math::Vec3;
use abrash_core::mesh::Mesh;

/// Generates a procedural 3D mesh of a Lorenz Attractor using numerical integration.
pub fn generate_lorenz_attractor(
    iterations: usize,
    dt: f32,
    sigma: f32,
    rho: f32,
    beta: f32,
) -> Mesh {
    let mut mesh = Mesh::with_capacity(iterations * 8, iterations * 12);

    // Initial state, close to origin
    let mut current_pos = Vec3::new(0.1, 0.0, 0.0);
    let radius = 0.5; // Radius of the tube segment

    for _ in 0..iterations {
        let x = current_pos.x;
        let y = current_pos.y;
        let z = current_pos.z;

        // Lorenz equations (Euler method integration)
        let dx = sigma * (y - x);
        let dy = x * (rho - z) - y;
        let dz = x * y - beta * z;

        let next_pos = Vec3::new(
            x + dx * dt,
            y + dy * dt,
            z + dz * dt,
        );

        add_segment(&mut mesh, current_pos, next_pos, radius);

        current_pos = next_pos;
    }

    mesh
}

fn add_segment(mesh: &mut Mesh, start: Vec3, end: Vec3, radius: f32) {
    let r = radius;
    // Calculate direction, up and right vectors
    let dir = (end - start).normalize();

    // To prevent NaNs when generating orthogonal vectors:
    let temp_up = if dir.x.abs() > 0.9 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };

    let right = dir.cross(temp_up).normalize();
    let up = right.cross(dir).normalize();

    let right_offset = right * r;
    let up_offset = up * r;

    let base_index = mesh.vertices.len();

    // Bottom vertices
    mesh.vertices.extend([
        start + right_offset, // 0
        start + up_offset,    // 1
        start - right_offset, // 2
        start - up_offset,    // 3
    ]);

    // Top vertices
    let right_top = right * r;
    let up_top = up * r;

    mesh.vertices.extend([
        end + right_top, // 4
        end + up_top,    // 5
        end - right_top, // 6
        end - up_top,    // 7
    ]);

    // Faces (Bottom, Top, Sides)
    mesh.indices.extend([
        [base_index, base_index + 2, base_index + 1],
        [base_index, base_index + 3, base_index + 2],
        [base_index + 4, base_index + 5, base_index + 6],
        [base_index + 4, base_index + 6, base_index + 7],
        [base_index, base_index + 1, base_index + 5],
        [base_index, base_index + 5, base_index + 4],
        [base_index + 1, base_index + 2, base_index + 6],
        [base_index + 1, base_index + 6, base_index + 5],
        [base_index + 2, base_index + 3, base_index + 7],
        [base_index + 2, base_index + 7, base_index + 6],
        [base_index + 3, base_index, base_index + 4],
        [base_index + 3, base_index + 4, base_index + 7],
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_lorenz_attractor() {
        let iterations = 100;
        let mesh = generate_lorenz_attractor(iterations, 0.01, 10.0, 28.0, 8.0 / 3.0);

        assert_eq!(mesh.vertices.len(), 800);
        assert_eq!(mesh.indices.len(), 1200); // 12 * 100 = 1200 triangles
    }
}
