use abrash::experimental::conway::{ConwayConfig, apply_conway};
use abrash::framebuffer::Framebuffer;
use abrash::geometry::{Cube, Mesh};
use abrash::math::Mat4;
use abrash::rasterizer::draw_mesh;
use abrash::zbuffer::ZBuffer;
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("🌟 Nova: Conway's Game of Life Filter")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)
        .unwrap();

    let context = unsafe { Context::new(&window) }.unwrap();
    let mut surface = unsafe { Surface::new(&context, &window) }.unwrap();

    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let mut cube_mesh = Cube::new(1.0).to_mesh();
    // Color it bright white so it seeds the Game of Life
    for v in &mut cube_mesh.vertices {
        v.color = 0xFF_FFFFFF;
    }

    let mut angle = 0.0;

    let config = ConwayConfig {
        cell_size: 6,
        seed_threshold: 128,
        live_color: 0xFF_00FF00,
        dead_color: 0xFF_000000,
        overlay: false,
        overlay_opacity: 1.0,
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                // Render frame
                fb.clear(0xFF_000000); // clear to black
                zb.clear();

                // Draw spinning cube
                let model = Mat4::rotation_x(angle) * Mat4::rotation_y(angle * 0.7);
                let view = Mat4::look_at(
                    abrash::math::Vec3::new(0.0, 0.0, 3.0),
                    abrash::math::Vec3::new(0.0, 0.0, 0.0),
                    abrash::math::Vec3::new(0.0, 1.0, 0.0),
                );
                let proj = Mat4::perspective(1.57, width as f32 / height as f32, 0.1, 100.0);

                let view_proj = view * proj;

                draw_mesh(&mut fb, &mut zb, &cube_mesh, model, view_proj);

                // Apply Game of Life
                apply_conway(&mut fb, &config);

                angle += 0.02;

                // Present
                surface
                    .resize(
                        NonZeroU32::new(width as u32).unwrap(),
                        NonZeroU32::new(height as u32).unwrap(),
                    )
                    .unwrap();

                let mut buffer = surface.buffer_mut().unwrap();
                buffer.copy_from_slice(fb.as_slice());
                buffer.present().unwrap();
            }
            _ => (),
        }
    });
}
