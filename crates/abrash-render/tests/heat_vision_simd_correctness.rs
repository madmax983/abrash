use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_render::heat_vision::apply_heat_vision;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_heat_vision_handles_extreme_depths(
        depths in proptest::collection::vec(
            proptest::num::f32::ANY,
            10..1000 // Test arrays of different lengths (scalar & SIMD paths)
        )
    ) {
        let width = depths.len() as u32;
        let height = 1;
        let mut fb = Framebuffer::new(width, height).unwrap();
        let mut zb = ZBuffer::new(width, height).unwrap();

        for (i, &d) in depths.iter().enumerate() {
            zb.test_and_set(i as i32, 0, d);
        }

        // Apply heat vision - ensuring it handles extreme/inf/NaN floats properly
        apply_heat_vision(&mut fb, &zb);

        // Verification: If all depths were infinity or NaN, it clears to background.
        // Otherwise, it computes colors. The crucial property is that it doesn't panic.
        let mut has_content = false;
        for &d in &depths {
            if d.is_finite() {
                has_content = true;
                break;
            }
        }

        if !has_content {
             // Fallback background color checking can be added if needed, but avoiding panic is the main goal
        }
    }
}
