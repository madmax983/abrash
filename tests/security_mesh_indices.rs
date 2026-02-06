#[cfg(test)]
mod tests {
    use abrash::math::Vec3;
    use abrash::mesh::Mesh;

    #[test]
    fn test_mesh_indices_error() {
        let mut mesh = Mesh::new();
        mesh.vertices.push(Vec3::new(0.0, 0.0, 0.0));
        // 1 and 2 are out of bounds (len is 1)
        mesh.indices.push([0, 1, 2]);

        let result = mesh.compute_face_normals();
        assert!(result.is_err());
        assert_eq!(result.err(), Some("Index out of bounds"));
    }
}
