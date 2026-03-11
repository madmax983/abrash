#[cfg(feature = "nova")]
use abrash::experimental::edge_glow::{EdgeGlowConfig, apply_edge_glow};
#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use abrash::platform::PlatformContext;

#[cfg(feature = "nova")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height)?;
    let mut ctx = PlatformContext::new("Edge Glow Demo", width, height)?;

    let config = EdgeGlowConfig {
        edge_color: 0x00_FF_00_FF, // Magenta
        intensity: 2.0,
        edge_threshold: 50,
        darken_factor: 0.2,
    };

    while ctx.update() {
        fb.clear(0xFF000000);
        let pixels = fb.as_mut_slice();
        for y in 0..height {
            for x in 0..width {
                let val = ((x + y) % 256) as u32;
                pixels[(y * width + x) as usize] = 0xFF00_0000 | (val << 16) | (val << 8) | val;
            }
        }

        apply_edge_glow(&mut fb, &config);

        ctx.draw_framebuffer(&fb)?;
    }

    Ok(())
}

#[cfg(not(feature = "nova"))]
fn main() {}
