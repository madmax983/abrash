#[cfg(test)]
mod tests {
    use abrash::camera::Camera;
    use abrash::math::Vec3;

    #[test]
    fn test_camera_creation() {
        let pos = Vec3::new(0.0, 0.0, 5.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let camera = Camera::new(pos, target, up, 1.0, 1.33, 0.1, 100.0);

        // Check if view matrix is calculated
        let _view = camera.view_matrix();
        let _proj = camera.projection_matrix();
        let _vp = camera.view_projection_matrix();
    }
}
