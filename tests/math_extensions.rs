#[cfg(test)]
mod tests {
    use abrash::math::Vec3;

    #[test]
    fn test_vec3_additions() {
        let v1 = Vec3::new(0.0, 0.0, 0.0);
        let v2 = Vec3::new(10.0, 10.0, 10.0);

        // Test lerp
        let v_lerp = v1.lerp(v2, 0.5);
        assert_eq!(v_lerp, Vec3::new(5.0, 5.0, 5.0));

        // Test length_sq
        let v3 = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(v3.length_sq(), 14.0);

        // Test Div<f32>
        let v4 = Vec3::new(10.0, 20.0, 30.0);
        let v_div = v4 / 2.0;
        assert_eq!(v_div, Vec3::new(5.0, 10.0, 15.0));
    }
}
