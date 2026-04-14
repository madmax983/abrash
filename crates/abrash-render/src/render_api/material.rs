//! Material definitions describing how surfaces are shaded.

use crate::render_api::handles::TextureHandle;

/// How a surface should be shaded.
///
/// This enum replaces the algorithm-first pattern of choosing between
/// `fill_triangle_flat`/`fill_triangle_gouraud`/`fill_triangle_phong`/`fill_triangle_textured`
/// at each call site. The renderer dispatches to the appropriate rasterization path
/// based on the shading mode.
#[derive(Debug, Clone)]
pub enum ShadingMode {
    /// Single color per triangle. Fastest mode.
    Flat {
        /// The flat color in `0xAARRGGBB` format.
        color: u32,
    },
    /// Per-vertex color interpolation.
    Gouraud,
    /// Per-pixel lighting with specular highlights.
    Phong {
        /// How sharp the specular highlight is (higher = sharper).
        shininess: f32,
        /// Intensity multiplier for specular reflections.
        specular_strength: f32,
    },
    /// Perspective-correct texture mapping.
    Textured {
        /// The texture to map onto the surface.
        texture: TextureHandle,
    },
    /// Texture mapping with per-vertex color modulation.
    TexturedGouraud {
        /// The texture to modulate with per-vertex color.
        texture: TextureHandle,
    },
    /// Physically-based rendering.
    Pbr {
        /// Base color map.
        albedo: TextureHandle,
        /// Surface roughness parameter (0.0 = smooth/mirror, 1.0 = rough/matte).
        roughness: f32,
        /// How metallic the surface is (0.0 = dielectric, 1.0 = metal).
        metallic: f32,
    },
    /// Environment/cubemap reflection.
    Reflection {
        /// The environment texture/cubemap.
        texture: TextureHandle,
        /// How reflective the surface is (0.0 = diffuse, 1.0 = perfect mirror).
        reflectivity: f32,
    },
    /// Normal-mapped with per-pixel lighting.
    NormalMapped {
        /// Base diffuse color map.
        diffuse: TextureHandle,
        /// Tangent-space normal map.
        normal_map: TextureHandle,
        /// Specular shininess exponent.
        shininess: f32,
    },
}

/// A material definition combining shading mode with rendering properties.
#[derive(Debug, Clone)]
pub struct Material {
    /// How this surface is shaded.
    pub shading: ShadingMode,
    /// Base color tint (multiplied with shading result). `0xAARRGGBB`.
    pub color: u32,
    /// Whether this material is affected by lighting.
    pub receive_light: bool,
}

impl Material {
    /// Create a simple flat-colored material.
    #[must_use]
    pub const fn flat(color: u32) -> Self {
        Self {
            shading: ShadingMode::Flat { color },
            color,
            receive_light: false,
        }
    }

    /// Create a Gouraud-shaded material.
    #[must_use]
    pub const fn gouraud(color: u32) -> Self {
        Self {
            shading: ShadingMode::Gouraud,
            color,
            receive_light: true,
        }
    }

    /// Create a Phong-shaded material.
    #[must_use]
    pub const fn phong(color: u32, shininess: f32, specular_strength: f32) -> Self {
        Self {
            shading: ShadingMode::Phong {
                shininess,
                specular_strength,
            },
            color,
            receive_light: true,
        }
    }

    /// Create a textured material.
    #[must_use]
    pub const fn textured(texture: TextureHandle) -> Self {
        Self {
            shading: ShadingMode::Textured { texture },
            color: 0xFFFF_FFFF,
            receive_light: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_api::handles::Handle;

    #[test]
    fn test_flat_material() {
        let mat = Material::flat(0xFFFF_0000);
        assert_eq!(mat.color, 0xFFFF_0000);
        assert!(!mat.receive_light);
        match mat.shading {
            ShadingMode::Flat { color } => assert_eq!(color, 0xFFFF_0000),
            _ => panic!("Expected Flat shading"),
        }
    }

    #[test]
    fn test_phong_material() {
        let mat = Material::phong(0xFFFF_FFFF, 32.0, 0.5);
        assert!(mat.receive_light);
        match mat.shading {
            ShadingMode::Phong {
                shininess,
                specular_strength,
            } => {
                assert!((shininess - 32.0).abs() < f32::EPSILON);
                assert!((specular_strength - 0.5).abs() < f32::EPSILON);
            }
            _ => panic!("Expected Phong shading"),
        }
    }

    #[test]
    fn test_textured_material() {
        let tex_handle: TextureHandle = Handle::new(0, 0);
        let mat = Material::textured(tex_handle);
        assert_eq!(mat.color, 0xFFFF_FFFF);
        assert!(!mat.receive_light);
    }
}
