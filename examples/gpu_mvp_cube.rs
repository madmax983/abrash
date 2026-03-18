//! GPU MVP cube demo using the new renderer/surface split.

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_gpu_render::renderer::GpuRenderer;
use abrash_render::render_api::frame::{Frame, FrameCamera};
use abrash_render::render_api::material::Material;
use std::sync::Arc;
use std::time::Instant;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn main() -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|error| format!("{error}"))?;
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Abrash GPU MVP Cube")
            .with_inner_size(PhysicalSize::new(1280u32, 720u32))
            .build(&event_loop)
            .map_err(|error| format!("{error}"))?,
    );

    let (mut renderer, mut surface) = GpuRenderer::new_windowed(window.clone())?;
    let mesh = renderer.create_mesh(&Mesh::cube(1.0))?;
    let material = renderer.create_material(Material::flat(0xFFFF_4444));
    let start = Instant::now();

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(size) => {
                    surface.resize(renderer.device(), size.width, size.height);
                }
                WindowEvent::RedrawRequested => {
                    let elapsed = start.elapsed().as_secs_f32();
                    let angle = elapsed * 0.8;
                    let size = window.inner_size();
                    let aspect = size.width.max(1) as f32 / size.height.max(1) as f32;
                    let camera = FrameCamera::new(
                        Mat4::look_at(
                            Vec3::new(0.0, 2.0, 5.0),
                            Vec3::ZERO,
                            Vec3::new(0.0, 1.0, 0.0),
                        ),
                        Mat4::perspective(1.0, aspect, 0.1, 100.0),
                    );
                    let mut frame = Frame::new(camera);
                    frame.draw(mesh, material, Mat4::rotation_y(angle));

                    if let Err(error) = renderer.render_to_surface(&frame, &surface) {
                        eprintln!("render error: {error}");
                    }
                }
                _ => {}
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        })
        .map_err(|error| format!("{error}"))
}
