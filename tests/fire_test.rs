#[cfg(feature = "nova")]
use abrash::experimental::fire::{FireEffect, render_fire};
use abrash::framebuffer::Framebuffer;

#[cfg(feature = "nova")]
#[test]
fn test_fire_effect() {
    let width = 64;
    let height = 64;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut fire = FireEffect::new(width, height);

    // Step 1: Fire buffer should initially be all zeros
    let buf = fire.buffer();
    assert!(buf.iter().all(|&v| v == 0), "Fire buffer not initialized to zero");

    // Step 2: Seed the bottom row and update
    fire.seed_bottom_row();
    fire.update();

    // Step 3: Render to framebuffer
    render_fire(&mut fb, &fire);

    // Step 4: Verify some pixels are drawn (not completely black)
    let mut has_fire = false;
    for y in 0..height {
        for x in 0..width {
            let pixel = fb.get_pixel(x as i32, y as i32).unwrap_or(0);
            if pixel != 0xFF000000 && pixel != 0 {
                has_fire = true;
                break;
            }
        }
    }

    assert!(has_fire, "Fire effect did not render any visible pixels");
}
