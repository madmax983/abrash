//! Procedural City Generation
//!
//! Generates a city mesh with buildings of varying heights.

use crate::mesh::Mesh;
use crate::utils::XorShift32;

/// Configuration for city generation.
pub struct CityConfig {
    pub width: f32,
    pub depth: f32,
    pub grid_size: usize,
    pub min_height: f32,
    pub max_height: f32,
    pub building_probability: f32,
    pub seed: u32,
}

impl Default for CityConfig {
    fn default() -> Self {
        Self {
            width: 100.0,
            depth: 100.0,
            grid_size: 10,
            min_height: 1.0,
            max_height: 5.0,
            building_probability: 0.7,
            seed: 12345,
        }
    }
}

/// Generates a city mesh based on the configuration.
#[must_use]
pub fn generate_city(config: &CityConfig) -> Mesh {
    let mut city_mesh = Mesh::new();
    let mut rng = XorShift32::new(config.seed);

    let cell_width = config.width / config.grid_size as f32;
    let cell_depth = config.depth / config.grid_size as f32;

    // Gap between buildings
    let padding = 0.1 * cell_width.min(cell_depth);
    let building_w = cell_width - padding * 2.0;
    let building_d = cell_depth - padding * 2.0;

    let start_x = -config.width * 0.5;
    let start_z = -config.depth * 0.5;

    for z in 0..config.grid_size {
        for x in 0..config.grid_size {
            // Decide if we place a building
            if rng.next_f32() > config.building_probability {
                continue;
            }

            let height =
                config.min_height + rng.next_f32() * (config.max_height - config.min_height);

            // Center of the cell
            let cx = start_x + (x as f32 * cell_width) + cell_width * 0.5;
            let cz = start_z + (z as f32 * cell_depth) + cell_depth * 0.5;

            // Building base is at y=0, so center y is height/2
            let cy = height * 0.5;

            // Create a building mesh (cube)
            let mut building = Mesh::cube(1.0);

            // Scale and Translate vertices
            for v in &mut building.vertices {
                v.x *= building_w;
                v.y *= height;
                v.z *= building_d;

                v.x += cx;
                v.y += cy; // Shift so bottom is at 0
                v.z += cz;
            }

            // Merge into city mesh
            merge_mesh(&mut city_mesh, &building);
        }
    }

    city_mesh
}

fn merge_mesh(target: &mut Mesh, source: &Mesh) {
    let vertex_offset = target.vertices.len();

    // Append vertices
    target.vertices.extend_from_slice(&source.vertices);
    target.uvs.extend_from_slice(&source.uvs);
    target.normals.extend_from_slice(&source.normals);
    target.tangents.extend_from_slice(&source.tangents);

    // Append indices with offset
    for tri in &source.indices {
        target.indices.push([
            tri[0] + vertex_offset,
            tri[1] + vertex_offset,
            tri[2] + vertex_offset,
        ]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_city_generation() {
        let config = CityConfig {
            grid_size: 2,
            building_probability: 1.0, // Force buildings
            ..Default::default()
        };

        let mesh = generate_city(&config);

        // 2x2 grid = 4 buildings.
        // Each building is a cube = 8 vertices.
        // Total vertices = 32.
        assert_eq!(mesh.vertices.len(), 32);

        // Each building = 12 triangles.
        // Total triangles = 48.
        assert_eq!(mesh.indices.len(), 48);
    }

    #[test]
    fn test_merge_mesh() {
        let mut m1 = Mesh::cube(1.0);
        let m2 = Mesh::cube(1.0);

        merge_mesh(&mut m1, &m2);

        assert_eq!(m1.vertices.len(), 16);
        assert_eq!(m1.indices.len(), 24);

        // Check indices offset for the second cube (vertices 8-15)
        let last_tri = m1.indices.last().unwrap();
        assert!(last_tri[0] >= 8);
        assert!(last_tri[1] >= 8);
        assert!(last_tri[2] >= 8);
    }
}
