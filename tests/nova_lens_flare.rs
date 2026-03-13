#![cfg(feature = "nova")]

use abrash::experimental::lens_flare::{apply_lens_flare, LensFlareConfig};
use abrash::framebuffer::Framebuffer;

#[test]
fn test_lens_flare_generation() -> Result<(), &'static str> {
    let mut fb = Framebuffer::new(100, 100)?;

    // Create a dark background
    fb.clear(0xFF101010); // Very dark gray

    // Place a very bright white spot in the top-left quadrant (x=20, y=20)
    // Center is (50, 50)
    for dy in -2..=2 {
        for dx in -2..=2 {
            fb.set_pixel(20 + dx, 20 + dy, 0xFFFFFFFF);
        }
    }

    // Apply the lens flare effect with light position at (20, 20)
    let config = LensFlareConfig::default();

    apply_lens_flare(&mut fb, &config, 20.0, 20.0);

    // Light is at (20,20), center is (50,50). Vector from light to center is (30,30).
    // offset = -0.5 + i * 0.4
    // ghosts = 5. Dispersal = 0.4.
    // i=0: offset = -0.5
    // i=1: offset = -0.1
    // i=2: offset = 0.3
    // i=3: offset = 0.7
    // i=4: offset = 1.1
    // ghost_x = 20 + 30 * offset
    // i=3: ghost_x = 20 + 21 = 41
    // i=4: ghost_x = 20 + 33 = 53
    // The ghosts will be spawned across the line passing through center!

    // Let's check the pixel near ghost i=3 (41, 41)
    let color = fb.as_slice()[(41 * 100 + 41) as usize];

    // The pixel should be significantly brighter than the background (0xFF101010)
    let r = (color >> 16) & 0xFF;
    assert!(r > 0x10, "Ghost reflection was not generated at the expected location. Found red channel {}", r);
    Ok(())
}

#[test]
fn test_lens_flare_zero_size() -> Result<(), &'static str> {
    let mut fb = Framebuffer::new(0, 0)?;
    let config = LensFlareConfig::default();
    apply_lens_flare(&mut fb, &config, 10.0, 10.0);
    assert_eq!(fb.width(), 0);
    Ok(())
}
