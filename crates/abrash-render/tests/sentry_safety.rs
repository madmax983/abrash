use abrash_render::render_api::RenderError;
use abrash_render::render_api::handles::ResourcePool;

#[test]
fn test_render_error_display() {
    let err = RenderError::InvalidMesh("Index out of bounds".to_string());
    assert!(err.to_string().contains("Index out of bounds"));

    let err2 = RenderError::StaleHandle("Mesh");
    assert!(err2.to_string().contains("Mesh"));
}

#[test]
fn test_resource_pool_capacity() {
    let pool: ResourcePool<u32> = ResourcePool::with_capacity(1);
    assert_eq!(pool.capacity(), 1);
}
