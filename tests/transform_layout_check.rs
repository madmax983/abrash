#[cfg(test)]
mod tests {
    use abrash::math::Vec3;
    use std::mem;

    #[test]
    fn test_vec3_f32_tuple_layout() {
        // Verify that (Vec3, f32) has the layout [x, y, z, w] with no padding
        // This is required for the AVX2 transform_points implementation which treats
        // the output buffer as a contiguous array of f32s.

        let v = Vec3::new(1.0, 2.0, 3.0);
        let w = 4.0f32;
        let tuple = (v, w);

        let size = mem::size_of::<(Vec3, f32)>();
        let align = mem::align_of::<(Vec3, f32)>();

        assert_eq!(size, 16, "(Vec3, f32) size should be 16 bytes");
        assert_eq!(align, 4, "(Vec3, f32) align should be 4 bytes");

        let ptr = (&raw const tuple).cast::<f32>();
        unsafe {
            assert_eq!((*ptr.add(0)).to_bits(), 1.0f32.to_bits());
            assert_eq!((*ptr.add(1)).to_bits(), 2.0f32.to_bits());
            assert_eq!((*ptr.add(2)).to_bits(), 3.0f32.to_bits());
            assert_eq!((*ptr.add(3)).to_bits(), 4.0f32.to_bits());
        }
    }
}
