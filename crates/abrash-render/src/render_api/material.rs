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
        /// Base color as `0xRRGGBB`.
        color: u32,
    },
    /// Per-vertex color interpolation.
    Gouraud,
    /// Per-pixel lighting with specular highlights.
    Phong {
        /// The specular exponent.
        shininess: f32,
        /// Strength of the specular highlight (0.0 to 1.0).
        specular_strength: f32,
    },
    /// Perspective-correct texture mapping.
    Textured {
        /// Handle to the diffuse texture.
        texture: TextureHandle,
    },
    /// Texture mapping with per-vertex color modulation.
    TexturedGouraud {
        /// Handle to the diffuse texture.
        texture: TextureHandle,
    },
    /// Physically-based rendering.
    Pbr {
        /// Handle to the albedo (diffuse) texture.
        albedo: TextureHandle,
        /// Material roughness (0.0 = smooth, 1.0 = rough).
        roughness: f32,
        /// Material metallic factor (0.0 = dielectric, 1.0 = metallic).
        metallic: f32,
    },
    /// Environment/cubemap reflection.
    Reflection {
        /// Handle to the reflection environment texture.
        texture: TextureHandle,
        /// Reflectivity factor (0.0 = matte, 1.0 = perfect mirror).
        reflectivity: f32,
    },
    /// Normal-mapped with per-pixel lighting.
    NormalMapped {
        /// Handle to the diffuse texture.
        diffuse: TextureHandle,
        /// Handle to the normal map texture.
        normal_map: TextureHandle,
        /// The specular exponent.
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
