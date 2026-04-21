use abrash::experimental::droste::{DrosteConfig, apply_droste};
use abrash::framebuffer::Framebuffer;
use abrash::platform::{AppResult, WindowApp, WindowContext};

struct DrosteDemo {
    config: DrosteConfig,
}

impl WindowApp for DrosteDemo {
    fn new(_ctx: WindowContext<'_>) -> AppResult<Self> {
        let mut config = DrosteConfig::default();
        config.inner_radius = 0.2;
        config.outer_radius = 1.0;
        config.spiral = true;
        config.arms = 2.0;

        Ok(Self { config })
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> AppResult<()> {
        self.config.time += ctx.dt_seconds * 0.1;
        Ok(())
    }

    fn render(&mut self, fb: &mut Framebuffer) -> AppResult<()> {
        // Draw a basic concentric pattern
        fb.clear(0xFF_22_22_22);

        let cx = fb.width() as i32 / 2;
        let cy = fb.height() as i32 / 2;

        // Draw some rings so the droste effect has something to distort
        for r in (50..300).step_by(20) {
            let color = if (r / 20) % 2 == 0 {
                0xFF_FF_44_44
            } else {
                0xFF_44_44_FF
            };
            for dy in -r..r {
                for dx in -r..r {
                    if dx * dx + dy * dy <= r * r && dx * dx + dy * dy >= (r - 5) * (r - 5) {
                        fb.set_pixel(cx + dx, cy + dy, color);
                    }
                }
            }
        }

        apply_droste(fb, &self.config);
        Ok(())
    }
}

fn main() {
    abrash::platform::run::<DrosteDemo>("Droste Effect Demo", 800, 600);
}
