//! Skybox rendering module.
//!
//! Implements a Skybox using a Cubemap texture.
//! The skybox is rendered as a unit cube centered on the camera,
//! with "infinite" depth (z=1.0) to serve as a background.

use crate::framebuffer::Framebuffer;
use crate::math::{Mat4, Vec2, Vec3};
use crate::rasterizer::texture::fill_quad_textured;
use crate::texture::Texture;
use crate::zbuffer::ZBuffer;

/// A Cubemap texture consisting of 6 faces.
///
/// Faces are ordered: +X, -X, +Y, -Y, +Z, -Z.
/// (Right, Left, Top, Bottom, Front, Back).
///
/// # Examples
///
/// ```
/// use abrash_core::texture::Texture;
/// use abrash_render::skybox::Cubemap;
///
/// let faces = [
///     Texture::new(1, 1).unwrap(),
///     Texture::new(1, 1).unwrap(),
///     Texture::new(1, 1).unwrap(),
///     Texture::new(1, 1).unwrap(),
///     Texture::new(1, 1).unwrap(),
///     Texture::new(1, 1).unwrap(),
/// ];
/// let cubemap = Cubemap::new(faces);
/// ```
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

/// Helper to render a Skybox.
///
/// This constructs a unit cube and rasterizes it using optimized quad rendering.
/// It modifies the view matrix to remove translation, ensuring the skybox stays centered.
///
/// # Examples
///
/// ```
/// use abrash_core::framebuffer::Framebuffer;
/// use abrash_core::zbuffer::ZBuffer;
/// use abrash_core::texture::Texture;
/// use abrash_core::math::Mat4;
/// use abrash_render::skybox::{Cubemap, draw_skybox};
///
/// let mut fb = Framebuffer::new(100, 100).unwrap();
/// let mut zb = ZBuffer::new(100, 100).unwrap();
///
/// let faces = [
///     Texture::new(1, 1).unwrap(),
///     Texture::new(1, 1).unwrap(),
///     Texture::new(1, 1).unwrap(),
///     Texture::new(1, 1).unwrap(),
///     Texture::new(1, 1).unwrap(),
///     Texture::new(1, 1).unwrap(),
/// ];
/// let cubemap = Cubemap::new(faces);
///
/// let view = Mat4::identity();
/// let proj = Mat4::identity();
///
/// draw_skybox(&mut fb, &mut zb, view, proj, &cubemap);
/// ```
pub fn draw_skybox(
    fb: &mut Framebuffer,
    zb: &mut ZBuffer,
    view: Mat4,
    proj: Mat4,
    cubemap: &Cubemap,
) {
    // 1. Remove translation from View Matrix
    let mut view_centered = view;
    view_centered.m[3][0] = 0.0;
    view_centered.m[3][1] = 0.0;
    view_centered.m[3][2] = 0.0;

    let view_proj = view_centered * proj;

    // Helper to process a face
    // Vertices order: TL, BL, BR, TR (from inside view)
    let mut draw_face = |face_idx: usize, verts: [Vec3; 4], uvs: [Vec2; 4]| {
        let mut projected = [((Vec3::default(), 0.0), Vec2::default()); 4];

        for i in 0..4 {
            // Transform
            let (mut p_clip, w) = view_proj.transform_point(verts[i]);
            // Force Z to W (Depth = 1.0)
            p_clip.z = w;
            projected[i] = ((p_clip, w), uvs[i]);
        }

        fill_quad_textured(
            fb,
            zb,
            projected[0],
            projected[1],
            projected[2],
            projected[3],
            &cubemap.faces[face_idx],
        );
    };

    // UV Constants (Mirrored U for standard skybox mapping)
    // U=1 is Left, U=0 is Right (due to 1-z logic typically)
    // Let's use the explicit mapping derived earlier
    let uv_tl = Vec2::new(1.0, 0.0);
    let uv_bl = Vec2::new(1.0, 1.0);
    let uv_br = Vec2::new(0.0, 1.0);
    let uv_tr = Vec2::new(0.0, 0.0);
    let uvs = [uv_tl, uv_bl, uv_br, uv_tr];

    // +X (Right)
    // TL (1, 1, -1), BL (1, -1, -1), BR (1, -1, 1), TR (1, 1, 1)
    draw_face(
        0,
        [
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ],
        uvs,
    );

    // -X (Left)
    // TL (-1, 1, 1), BL (-1, -1, 1), BR (-1, -1, -1), TR (-1, 1, -1)
    draw_face(
        1,
        [
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
        ],
        uvs,
    );

    // +Y (Top)
    // TL (1, 1, -1), BL (1, 1, 1), BR (-1, 1, 1), TR (-1, 1, -1)
    // Note: Winding derived earlier: (1, 1, -1), (-1, 1, -1), (-1, 1, 1), (1, 1, 1) was CW (Down).
    // Order TL->TR->BR->BL produced Down Normal.
    // The quad function expects TL, BL, BR, TR (CCW on screen).
    // So if (1, 1, -1) is TL and (-1, 1, -1) is TR.
    // And we want TL -> BL -> BR -> TR.
    // Vertices:
    // TL: (1, 1, -1)
    // BL: (1, 1, 1)
    // BR: (-1, 1, 1)
    // TR: (-1, 1, -1)
    // This order maps to uvs: (1,0), (1,1), (0,1), (0,0).
    draw_face(
        2,
        [
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, -1.0),
        ],
        uvs,
    );

    // -Y (Bottom)
    // TL (1, -1, 1), BL (1, -1, -1), BR (-1, -1, -1), TR (-1, -1, 1)
    draw_face(
        3,
        [
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
        ],
        uvs,
    );

    // +Z (Front)
    // TL (1, 1, 1), BL (1, -1, 1), BR (-1, -1, 1), TR (-1, 1, 1)
    draw_face(
        4,
        [
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ],
        uvs,
    );

    // -Z (Back)
    // TL (-1, 1, -1), BL (-1, -1, -1), BR (1, -1, -1), TR (1, 1, -1)
    // Note: Back face is looking at -Z.
    // Left is +X (-1 maps to u=1?). Wait.
    // sample logic for -Z: sc = -dir.x.
    // u = (-x+1)/2.
    // Left (x=-1) -> u=1.
    // Right (x=1) -> u=0.
    // So TL (x=-1) -> u=1.
    // Vertices:
    // TL (-1, 1, -1). u=1.
    // BL (-1, -1, -1). u=1.
    // BR (1, -1, -1). u=0.
    // TR (1, 1, -1). u=0.
    draw_face(
        5,
        [
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
        ],
        uvs,
    );
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

    #[test]
    fn test_draw_skybox_integration() {
        let width = 10;
        let height = 10;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        let faces = [
            Texture::new(1, 1).unwrap(),
            Texture::new(1, 1).unwrap(),
            Texture::new(1, 1).unwrap(),
            Texture::new(1, 1).unwrap(),
            Texture::new(1, 1).unwrap(),
            Texture::new(1, 1).unwrap(),
        ];
        let cubemap = Cubemap::new(faces);

        let view = Mat4::identity();
        let proj = Mat4::perspective(1.57, 1.0, 0.1, 100.0);

        // Should not panic
        draw_skybox(&mut fb, &mut zb, view, proj, &cubemap);
    }
}
