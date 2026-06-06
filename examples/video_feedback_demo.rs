//! Video Feedback Demo
//!
//! Demonstrates the experimental Video Feedback filter. A moving
//! shape leaves an organic trail that warps over time.

use abrash::experimental::video_feedback::{VideoFeedback, VideoFeedbackConfig};
use abrash::framebuffer::Framebuffer;
use abrash::math::Vec3;
use abrash::rasterizer::fill_triangle_3d;
use abrash::zbuffer::ZBuffer;

#[cfg(any(feature = "backend-winit", feature = "backend-tui"))]
use abrash::platform::{
    HostError, SoftwarePresenter, WindowApp, WindowContext, WindowHostConfig, run_windowed,
};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 480;
const TITLE: &str = "🌟 Nova: Video Feedback Demo";

#[cfg(any(feature = "backend-winit", feature = "backend-tui"))]
struct VideoFeedbackDemoApp {
    presenter: Option<SoftwarePresenter>,
    framebuffer: Framebuffer,
    zbuffer: ZBuffer,
    config: VideoFeedbackConfig,
    video_feedback: VideoFeedback,
    time: f32,
}

#[cfg(any(feature = "backend-winit", feature = "backend-tui"))]
impl VideoFeedbackDemoApp {
    fn new() -> Result<Self, HostError> {
        Ok(Self {
            presenter: None,
            framebuffer: Framebuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            zbuffer: ZBuffer::new(WIDTH, HEIGHT)
                .map_err(|error| HostError::App(error.to_string()))?,
            config: VideoFeedbackConfig {
                decay: 0.95,
                scale: 1.05,
                rotation: 0.05,
                offset_x: 0.0,
                offset_y: 0.0,
            },
            video_feedback: VideoFeedback::new(),
            time: 0.0,
        })
    }

    fn present(&mut self) -> Result<(), HostError> {
        let framebuffer = &self.framebuffer;
        let presenter = self
            .presenter
            .as_mut()
            .ok_or_else(|| HostError::Present("software presenter not initialized".to_string()))?;
        presenter.present(framebuffer)?;
        Ok(())
    }
}

#[cfg(any(feature = "backend-winit", feature = "backend-tui"))]
impl WindowApp for VideoFeedbackDemoApp {
    type Error = HostError;

    fn config(&self) -> WindowHostConfig {
        WindowHostConfig {
            title: TITLE.to_string(),
            width: WIDTH,
            height: HEIGHT,
            vsync: true,
        }
    }

    fn init(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.presenter = Some(SoftwarePresenter::new(ctx.window)?);
        Ok(())
    }

    fn update(&mut self, ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        self.time += ctx.dt_seconds.max(0.0);
        Ok(())
    }

    fn render(&mut self, _ctx: WindowContext<'_>) -> Result<(), Self::Error> {
        // Clear the background, but the video feedback effect will blend the history over it.
        // It's important to only draw the *new* frame contents on top of the black background,
        // and then apply video feedback to merge it with the history.
        self.framebuffer.clear(0xFF00_0000);
        self.zbuffer.clear();

        // Draw a simple moving triangle
        let cx = (self.time * 2.0).cos() * 100.0 + (WIDTH as f32 / 2.0);
        let cy = (self.time * 3.0).sin() * 80.0 + (HEIGHT as f32 / 2.0);

        let s = 40.0;
        let t = self.time * 5.0;

        // Wrap the vectors in tuples with w-component for perspective-correct rasterizer
        let v0 = (Vec3::new(cx + s * t.cos(), cy + s * t.sin(), 1.0), 1.0);
        let v1 = (
            Vec3::new(cx + s * (t + 2.09).cos(), cy + s * (t + 2.09).sin(), 1.0),
            1.0,
        );
        let v2 = (
            Vec3::new(cx + s * (t + 4.18).cos(), cy + s * (t + 4.18).sin(), 1.0),
            1.0,
        );

        // Use an additive glowing color
        fill_triangle_3d(
            &mut self.framebuffer,
            &mut self.zbuffer,
            v0,
            v1,
            v2,
            0xFFFF_5555,
        );

        // Apply video feedback, causing the trail to zoom outward and rotate
        self.video_feedback
            .apply(&mut self.framebuffer, &self.config);

        self.present()
    }
}

fn main() {
    #[cfg(any(feature = "backend-winit", feature = "backend-tui"))]
    run_windowed(VideoFeedbackDemoApp::new().unwrap());

    #[cfg(not(any(feature = "backend-winit", feature = "backend-tui")))]
    println!("Please enable the `backend-winit` or `backend-tui` feature to run this demo.");
}
