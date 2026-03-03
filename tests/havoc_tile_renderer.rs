use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::ClipTriangle;
use abrash::rasterizer::tile::TileRenderer;
use abrash::zbuffer::ZBuffer;

#[test]
fn test_havoc_sendptr_overlap() {
    let width = 100;
    let height = 100;

    // We want to force a condition where SendPtr accesses memory it shouldn't.
    // TileRenderer processes triangles in 32x32 tiles.
    // If a triangle spans multiple tiles, it puts the triangle into multiple tile bins.
    // But does it write to out-of-bounds pixels in a given tile?
    // Let's create an adversarial triangle that is extremely large but clipped or somehow bypasses proper binning.

    // Actually, SendPtr writes based on `fb_ptr.write(y * width + x, color)`.
    // It assumes that `render_single_tile` strictly only processes pixels within `[tx*32, (tx+1)*32]`.
    // Let's look at `render_single_tile`.
}
