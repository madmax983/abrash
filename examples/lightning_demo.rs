use abrash::experimental::lightning::{LightningBolt, apply_lightning};
use abrash::framebuffer::Framebuffer;
use abrash::math::Vec2;

#[cfg(feature = "backend-winit")]
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

#[cfg(feature = "backend-winit")]
struct LightningDemo {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    frame_count: u64,
}

#[cfg(feature = "backend-winit")]
impl LightningDemo {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(800, 600).map_err(|e| HostError::App(e.to_string()))?,
            frame_count: 0,
        })
    }
}

#[cfg(feature = "backend-winit")]
impl WindowApp for LightningDemo {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: "Nova: Lightning Demo".to_string(),
            width: self.framebuffer.width(),
            height: self.framebuffer.height(),
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn resize(
        &mut self,
        _ctx: WindowContext<'_>,
        width: u32,
        height: u32,
    ) -> Result<(), Self::Error> {
        self.framebuffer =
            Framebuffer::new(width, height).map_err(|e| HostError::App(e.to_string()))?;
        Ok(())
    }

    fn update(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.frame_count += 1;
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Clear to dark blue
        self.framebuffer.clear(0xFF00_0011);

        let cx = self.framebuffer.width() as f32 / 2.0;
        let cy = self.framebuffer.height() as f32 / 2.0;

        let start1 = Vec2::new(cx, cy - 200.0);
        let end1 = Vec2::new(cx - 200.0, cy + 200.0);
        let end2 = Vec2::new(cx + 200.0, cy + 200.0);

        // Slightly change seed over time to animate the lightning
        let seed = self.frame_count / 3;

        // Generate multiple branches
        let bolt1 = LightningBolt::generate(start1, end1, 0xFFAA_AAFF, 0.6, 6, 4.0, seed);
        let bolt2 = LightningBolt::generate(start1, end2, 0xFFAA_AAFF, 0.6, 6, 4.0, seed + 100);

        apply_lightning(&mut self.framebuffer, &bolt1);
        apply_lightning(&mut self.framebuffer, &bolt2);

        // Core inner glow
        let bolt1_core = LightningBolt::generate(start1, end1, 0xFFFF_FFFF, 0.6, 6, 1.5, seed);
        let bolt2_core =
            LightningBolt::generate(start1, end2, 0xFFFF_FFFF, 0.6, 6, 1.5, seed + 100);
        apply_lightning(&mut self.framebuffer, &bolt1_core);
        apply_lightning(&mut self.framebuffer, &bolt2_core);

        if let Some(presenter) = &mut self.presenter {
            presenter.present(&self.framebuffer)?;
        }
        Ok(())
    }
}

fn main() {
    #[cfg(feature = "backend-winit")]
    {
        if let Ok(app) = LightningDemo::new() {
            run_windowed(app);
        }
    }
}
