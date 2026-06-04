use abrash_core::mesh::Mesh;
use abrash_core::math::{Mat4};
use abrash_render::render_api::{Frame, Material, FrameCamera};
use abrash_render::render_api::cpu_renderer::CpuRenderer;

#[test]
fn test_parallel_stale_handle_panics() {
    let mut renderer = CpuRenderer::new(100, 100);

    let mesh_h = renderer.create_mesh(&Mesh::cube(1.0)).unwrap();
    let mat_h = renderer.create_material(Material::flat(0xFFFF_0000)).unwrap();

    let mut frame = Frame::new(FrameCamera::new(
        Mat4::identity(),
        Mat4::perspective(1.57, 1.0, 0.1, 100.0),
    ));
    frame.draw(mesh_h, mat_h, Mat4::identity());

    // Destroy the mesh to make the handle stale
    renderer.destroy_mesh(mesh_h);

    // This should return an error, but in parallel mode it currently panics!
    let result = renderer.extract_draw_list(&frame);
    assert!(result.is_err(), "Expected an error");
}
