#![allow(clippy::unreadable_literal)]
//! Correctness tests for software two-level hierarchical binning
//!
//! Verifies that software-based two-level binning (Coarse Bins -> Fine Tiles)
//! produces identical results to single-level binning.

use abrash::{
    framebuffer::Framebuffer,
    math::Vec3,
    tile_renderer::{ClipTriangle, TileRenderer},
    zbuffer::ZBuffer,
};

/// Helper to create a clip-space vertex
const fn make_clip_vertex(x: f32, y: f32, z: f32, w: f32) -> (Vec3, f32) {
    (Vec3::new(x, y, z), w)
}

/// Helper to create a simple test scene with known occlusion
fn create_occluder_scene() -> Vec<ClipTriangle> {
    vec![
        // Occluder: Large front triangle covering center
        (
            make_clip_vertex(-0.6, -0.6, 0.2, 1.0), // Close to camera
            make_clip_vertex(0.6, -0.6, 0.2, 1.0),
            make_clip_vertex(0.0, 0.6, 0.2, 1.0),
            0xFF0000FF, // Red
        ),
        // Occluded: Triangle behind the occluder
        (
            make_clip_vertex(-0.4, -0.4, 0.8, 1.0), // Far from camera
            make_clip_vertex(0.4, -0.4, 0.8, 1.0),
            make_clip_vertex(0.0, 0.4, 0.8, 1.0),
            0x00FF00FF, // Green (should be hidden)
        ),
    ]
}

#[test]
fn test_software_two_level_matches_single_level() {
    let width = 640;
    let height = 480;

    // Create renderers
    let mut renderer_single = TileRenderer::new(width, height);
    let mut renderer_two_level = TileRenderer::new(width, height);

    // Enable two-level binning for the second renderer
    // This method doesn't exist yet, so this is the "Red" phase
    renderer_two_level.enable_software_two_level_binning();

    // Create framebuffers
    let mut fb_single = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut fb_two_level = Framebuffer::new(width, height).expect("Failed to create framebuffer");
    let mut zb_single = ZBuffer::new(width, height).expect("Failed to create zbuffer");
    let mut zb_two_level = ZBuffer::new(width, height).expect("Failed to create zbuffer");

    // Test with simple scene
    let triangles = create_occluder_scene();

    // Render with both approaches
    renderer_single.render_batch(&mut fb_single, &mut zb_single, &triangles);
    renderer_two_level.render_batch(&mut fb_two_level, &mut zb_two_level, &triangles);

    // Compare results - should be pixel-identical
    let single_pixels = fb_single.as_slice();
    let two_level_pixels = fb_two_level.as_slice();

    let mut diff_count = 0;
    for i in 0..single_pixels.len() {
        if single_pixels[i] != two_level_pixels[i] {
            diff_count += 1;
        }
    }

    assert_eq!(
        diff_count, 0,
        "Two-level and single-level rendering differ in {diff_count} pixels"
    );
}
