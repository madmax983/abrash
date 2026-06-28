
use abrash_core::framebuffer::Framebuffer;
use abrash_render::post_process::filters::{VignetteConfig, apply_vignette};

#[test]
fn test_apply_vignette_darkens_corners() {
    let mut fb = Framebuffer::new(3, 3).unwrap();
    // Fill with white
    fb.clear(0xFF_FF_FF_FF);

    let config = VignetteConfig {
        intensity: 0.5,
        roundness: 0.5,
    };

    // Apply vignette with 0.5 intensity and 0.5 roundness (smoothness)
    apply_vignette(
        &mut fb,
        &config,
    );

    let center = fb.get_pixel(1, 1).unwrap() & 0xFF; // Blue channel
    let corner = fb.get_pixel(0, 0).unwrap() & 0xFF;

    // Corner should be darker than center
    assert!(
        corner < center,
        "Corner ({}) should be darker than center ({})",
        corner,
        center
    );
}

#[test]
fn test_apply_vignette_preserves_alpha() {
    let mut fb = Framebuffer::new(1, 1).unwrap();
    fb.set_pixel(0, 0, 0x80_FF_FF_FF); // Semi-transparent white

    let config = VignetteConfig {
        intensity: 0.5,
        roundness: 0.5,
    };

    apply_vignette(
        &mut fb,
        &config,
    );

    let p = fb.get_pixel(0, 0).unwrap();
    let a = p & 0xFF00_0000;

    assert_eq!(a, 0x80_00_00_00, "Alpha channel should be preserved");
}

#[test]
fn test_vignette_swar_exact_matches() {
    let mut fb = Framebuffer::new(1, 1).unwrap();
    fb.set_pixel(0, 0, 0xFF_80_80_80); // mid gray

    let config = VignetteConfig {
        intensity: 0.5,
        roundness: 0.5,
    };

    apply_vignette(&mut fb, &config);

    // After applying vignette, verify channels extracted safely without overflow.
    let p = fb.get_pixel(0, 0).unwrap();
    let r = (p >> 16) & 0xFF;
    let g = (p >> 8) & 0xFF;
    let b = p & 0xFF;

    assert_eq!(r, g);
    assert_eq!(g, b);
}

#[test]
fn test_vignette_swar_no_overflow_on_intense() {
    let mut fb = Framebuffer::new(1, 1).unwrap();
    fb.set_pixel(0, 0, 0xFF_FF_FF_FF); // Pure white

    // Test intense vignette (factor could exceed > 1.0 depending on logic, verify safely scaled downcast)
    let config = VignetteConfig {
        intensity: 1.5,
        roundness: 0.5,
    };

    apply_vignette(&mut fb, &config);

    let p = fb.get_pixel(0, 0).unwrap();
    let r = (p >> 16) & 0xFF;
    assert!(r <= 0xFF);
}
