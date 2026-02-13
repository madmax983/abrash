#[cfg(test)]
mod tests {
    use abrash::math::{Vec3, Vec4, Vec2};
    use abrash::mesh::Mesh;
    use abrash::rasterizer::fill_triangle_normal_mapped;
    use abrash::framebuffer::Framebuffer;
    use abrash::zbuffer::ZBuffer;
    use abrash::texture::Texture;

    #[test]
    fn test_vec4_exists() {
        let v = Vec4::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(v.x, 1.0);
        assert_eq!(v.w, 4.0);
    }

    #[test]
    fn test_mesh_has_normals_and_tangents() {
        let mesh = Mesh::new();
        // These fields should exist
        assert_eq!(mesh.normals.len(), 0);
        assert_eq!(mesh.tangents.len(), 0);
    }

    #[test]
    fn test_fill_triangle_normal_mapped_exists() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let mut zb = ZBuffer::new(100, 100).unwrap();
        let texture = Texture::checkered(32, 32, 0xFFFFFFFF, 0xFF000000).unwrap();
        let normal_map = Texture::checkered(32, 32, 0xFF8080FF, 0xFF8080FF).unwrap(); // Flat normal map

        let v0 = ((Vec3::new(0.0, 0.0, 0.0), 1.0), Vec2::new(0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec4::new(1.0, 0.0, 0.0, 1.0));
        let v1 = ((Vec3::new(1.0, 0.0, 0.0), 1.0), Vec2::new(1.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec4::new(1.0, 0.0, 0.0, 1.0));
        let v2 = ((Vec3::new(0.0, 1.0, 0.0), 1.0), Vec2::new(0.0, 1.0), Vec3::new(0.0, 0.0, 1.0), Vec4::new(1.0, 0.0, 0.0, 1.0));

        fill_triangle_normal_mapped(
            &mut fb,
            &mut zb,
            v0,
            v1,
            v2,
            &texture,
            &normal_map,
            Vec3::new(0.0, 0.0, -1.0), // Light dir
            Vec3::new(1.0, 1.0, 1.0),  // Light color
            Vec3::new(0.1, 0.1, 0.1),  // Ambient
        );
    }
}
