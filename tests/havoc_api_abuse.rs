use abrash::texture::Texture;
use abrash::tile_renderer::TileRenderer;

#[test]
#[should_panic(expected = "Dimensions must be positive")]
fn crash_tile_renderer_zero_dim() {
    // 🧨 Trigger: Zero dimensions cause assertion failure (panic)
    // Denial of Service in library consumers.
    let _tr = TileRenderer::new(0, 100);
}

#[test]
#[should_panic(expected = "Texture dimensions must be positive")]
fn crash_texture_zero_dim() {
    // 🧨 Trigger: Zero dimensions cause panic
    let _tex = Texture::new(0, 100).unwrap();
}

#[test]
#[should_panic(expected = "memory allocation failed")]
#[ignore] // Don't run by default as it consumes machine resources
fn crash_texture_oom() {
    // 🧨 Trigger: Allocate 16GB texture
    // Denial of Service via OOM.
    // 65536 * 65536 = 4G pixels. * 4 bytes = 16GB.
    let _tex = Texture::new(65536, 65536).unwrap();
}
