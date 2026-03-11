#[cfg(feature = "nova")]
use abrash::experimental::emboss::apply_emboss;
#[cfg(feature = "nova")]
use abrash::framebuffer::Framebuffer;
#[cfg(feature = "nova")]
use abrash::platform::PlatformContext;

#[cfg(feature = "nova")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height)?;
    let mut ctx = PlatformContext::new("Emboss Demo", width, height)?;

    while ctx.update() {
        fb.clear(0xFFFFFFFF);
        let pixels = fb.as_mut_slice();
        for y in 0..height {
            for x in 0..width {
                let color = if (x / 20 + y / 20) % 2 == 0 {
                    0xFF000000
                } else {
                    0xFFFFFFFF
                };
                pixels[(y * width + x) as usize] = color;
            }
        }

        apply_emboss(&mut fb);

        ctx.draw_framebuffer(&fb)?;
    }

    Ok(())
}

#[cfg(not(feature = "nova"))]
fn main() {}
