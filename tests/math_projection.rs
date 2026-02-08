#[cfg(test)]
mod tests {
    use abrash::math::{Vec3, project_to_screen};

    #[test]
    fn test_project_to_screen_normal() {
        let v = Vec3::new(0.0, 0.0, 5.0);
        let w = 1.0;
        let width = 100;
        let height = 100;
        let p = project_to_screen(v, w, width, height);

        // Center of screen
        assert_eq!(p.x, 50);
        assert_eq!(p.y, 50);
        assert!((p.z - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_project_to_screen_edge() {
        // NDC x = 1.0 -> screen x = width
        let v = Vec3::new(1.0, 0.0, 5.0);
        let w = 1.0;
        let width = 100;
        let height = 100;
        let p = project_to_screen(v, w, width, height);

        // (1.0 + 1.0) * 0.5 * 100 = 100.0 -> 100
        assert_eq!(p.x, 100);
        assert_eq!(p.y, 50);
    }

    #[test]
    fn test_project_to_screen_small_w() {
        // w < 0.0001 -> inv_w = 1.0
        let v = Vec3::new(10.0, 10.0, 5.0);
        let w = 0.00005; // Less than epsilon
        let width = 100;
        let height = 100;
        let p = project_to_screen(v, w, width, height);

        // Should behave as if inv_w = 1.0
        // ndc_x = 10.0 * 1.0 = 10.0
        // screen_x = (10.0 + 1.0) * 0.5 * 100 = 550
        assert_eq!(p.x, 550);
        // ndc_y = 10.0
        // screen_y = (1.0 - 10.0) * 0.5 * 100 = -450
        assert_eq!(p.y, -450);
    }

    #[test]
    fn test_project_to_screen_negative_w() {
        // Points behind camera
        let v = Vec3::new(0.0, 0.0, -5.0);
        let w = -1.0;
        let width = 100;
        let height = 100;
        let p = project_to_screen(v, w, width, height);

        // inv_w = -1.0
        // ndc_x = 0.0
        // ndc_y = 0.0
        // depth = -5.0 * -1.0 = 5.0
        assert_eq!(p.x, 50);
        assert_eq!(p.y, 50);
        assert!((p.z - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_project_to_screen_saturation() {
        // Very large coordinates saturating i32
        let v = Vec3::new(1e30, 0.0, 5.0);
        let w = 1.0;
        let width = 100;
        let height = 100;
        let p = project_to_screen(v, w, width, height);

        // Should saturate to i32::MAX
        assert_eq!(p.x, i32::MAX);

        let v_neg = Vec3::new(-1e30, 0.0, 5.0);
        let p_neg = project_to_screen(v_neg, w, width, height);

        // Should saturate to i32::MIN
        assert_eq!(p_neg.x, i32::MIN);
    }

    #[test]
    fn test_vec3_normalize_small_vector_behavior() {
        // Verify the specific behavior documented in journal
        let v = Vec3::new(0.00001, 0.0, 0.0);
        let n = v.normalize();

        // Length is 0.00001, which is < 0.0001.
        // Should return original vector, NOT normalized.
        assert_eq!(n.x, 0.00001);
        assert_eq!(n.y, 0.0);
        assert_eq!(n.z, 0.0);
    }
}
