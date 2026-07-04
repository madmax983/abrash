//! Skybox data types.

use crate::math::Vec3;
use crate::texture::Texture;

/// A Cubemap texture consisting of 6 faces.
///
/// Faces are ordered: +X, -X, +Y, -Y, +Z, -Z.
/// (Right, Left, Top, Bottom, Front, Back).
pub struct Cubemap {
    /// The 6 textures that make up the cubemap faces (Right, Left, Top, Bottom, Front, Back).
    pub faces: [Texture; 6],
}

impl Cubemap {
    /// Create a new Cubemap from 6 textures.
    #[must_use]
    pub const fn new(faces: [Texture; 6]) -> Self {
        Self { faces }
    }

    /// Sample the cubemap using a direction vector.
    /// Used for CPU-side ray tracing or reference.
    #[must_use]
    pub fn sample(&self, dir: Vec3) -> u32 {
        let abs_x = dir.x.abs();
        let abs_y = dir.y.abs();
        let abs_z = dir.z.abs();

        let (face_idx, ma, sc, tc) = if abs_x >= abs_y && abs_x >= abs_z {
            let ma = abs_x;
            if dir.x > 0.0 {
                (0, ma, -dir.z, -dir.y) // +X (Right)
            } else {
                (1, ma, dir.z, -dir.y) // -X (Left)
            }
        } else if abs_y >= abs_x && abs_y >= abs_z {
            let ma = abs_y;
            if dir.y > 0.0 {
                (2, ma, dir.x, dir.z) // +Y (Top)
            } else {
                (3, ma, dir.x, -dir.z) // -Y (Bottom)
            }
        } else {
            let ma = abs_z;
            if dir.z > 0.0 {
                (4, ma, dir.x, -dir.y) // +Z (Front)
            } else {
                (5, ma, -dir.x, -dir.y) // -Z (Back)
            }
        };

        // Avoid division by zero
        if ma == 0.0 {
            return 0xFF00_0000;
        }

        // Map to [0, 1]
        let u = (sc / ma + 1.0) * 0.5;
        let v = (tc / ma + 1.0) * 0.5;

        self.faces[face_idx].get_pixel_bilinear(u, v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cubemap_sampling_directions() {
        // Create dummy texture with 1 pixel
        let white_tex = || {
            let mut t = Texture::new(1, 1).unwrap();
            t.pixels[0] = 0xFFFF_FFFF;
            t
        };
        let black_tex = || Texture::new(1, 1).unwrap();

        // Map +X (Right) to White, others Black
        let faces = [
            white_tex(), // +X
            black_tex(),
            black_tex(),
            black_tex(),
            black_tex(),
            black_tex(),
        ];
        let cubemap = Cubemap::new(faces);

        // Sample +X direction
        let color = cubemap.sample(Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(color, 0xFFFF_FFFF);

        // Sample -X direction
        let color = cubemap.sample(Vec3::new(-1.0, 0.0, 0.0));
        assert_eq!(color, 0xFF00_0000);
    }
}
