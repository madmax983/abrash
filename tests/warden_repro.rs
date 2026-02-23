use abrash::math::{Vec3, project_to_screen_optimized};

#[test]
fn reproduce_project_to_screen_min_int() {
    let v = Vec3::new(-1e10, 0.0, 5.0);
    let w = 1.0;
    let half_width = 50.0;
    let half_height = 50.0;

    let p = project_to_screen_optimized(v, w, half_width, half_height);

    // This confirms that it returns the clamped value (-2147483520), preventing the vulnerability.
    assert_eq!(p.x, -2_147_483_520);
}
