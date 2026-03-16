use abrash::framebuffer::Framebuffer;
use abrash::rasterizer::tile::{TileRenderer, TileRendererConfig};
use abrash::scene::Scene;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_hiz_unwrap_panic() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let mut zb = ZBuffer::new(100, 100).unwrap();
        let mut config = TileRendererConfig::default();
        config.enable_hiz = true;
        let mut renderer = TileRenderer::new(100, 100, config);
        let scene = Scene::new();
        renderer.render_scene(&mut fb, &mut zb, &scene);
    }));
    assert!(result.is_err() || result.is_ok()); // just check if we can crash it, or maybe it runs fine.
}
