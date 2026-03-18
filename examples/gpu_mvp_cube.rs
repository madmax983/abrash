//! GPU MVP cube demo using the new renderer/surface split.

use abrash_core::math::{Mat4, Vec3};
use abrash_core::mesh::Mesh;
use abrash_gpu_render::renderer::GpuRenderer;
use abrash_render::render_api::frame::{Frame, FrameCamera};
use abrash_render::render_api::material::Material;
use comfy_table::{Cell, Color, Table, presets};
use crossterm::style::Stylize;
use std::sync::Arc;
use std::time::Instant;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn print_banner() {
    println!("\n{}", "🧊 GPU MVP Cube Demo".bold().cyan());
    println!("{}", "=====================".dark_grey());
}

fn print_error_table(error_msg: &str) {
    let mut error_table = Table::new();
    error_table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("❌ GPU Error")
                .add_attribute(comfy_table::Attribute::Bold)
                .fg(Color::Red),
        ])
        .add_row(vec![Cell::new(error_msg).fg(Color::Yellow)]);
    eprintln!("\n{error_table}");
}

fn print_info_table() {
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Property").fg(Color::Cyan),
            Cell::new("Value").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Description"),
            Cell::new("Hardware-accelerated MVP cube rendering").fg(Color::Yellow),
        ])
        .add_row(vec![
            Cell::new("Backend"),
            Cell::new("wgpu (Metal/Vulkan/DX12)").fg(Color::Green),
        ]);

    println!("\n{}", "⚙️  Info".bold());
    println!("{table}");
}

fn print_controls_table() {
    let mut controls = Table::new();
    controls
        .load_preset(presets::UTF8_FULL)
        .set_header(vec![
            Cell::new("Input").fg(Color::Cyan),
            Cell::new("Action").fg(Color::Cyan),
        ])
        .add_row(vec![
            Cell::new("Mouse"),
            Cell::new("None"),
        ])
        .add_row(vec![
            Cell::new("Keyboard"),
            Cell::new("Auto-rotating"),
        ]);

    println!("\n{}", "🎮 Controls".bold());
    println!("{controls}\n");
}

fn main() -> Result<(), String> {
    print_banner();
    print_info_table();
    print_controls_table();

    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(error) => {
            print_error_table(&format!("{error}"));
            std::process::exit(1);
        }
    };
    let window = Arc::new(
        match WindowBuilder::new()
            .with_title("Abrash GPU MVP Cube")
            .with_inner_size(PhysicalSize::new(1280u32, 720u32))
            .build(&event_loop)
        {
            Ok(w) => w,
            Err(error) => {
                print_error_table(&format!("{error}"));
                std::process::exit(1);
            }
        },
    );

    let (mut renderer, mut surface) = match GpuRenderer::new_windowed(window.clone()) {
        Ok(res) => res,
        Err(e) => {
            print_error_table(&e);
            std::process::exit(1);
        }
    };

    let mesh = match renderer.create_mesh(&Mesh::cube(1.0)) {
        Ok(m) => m,
        Err(e) => {
            print_error_table(&e);
            std::process::exit(1);
        }
    };

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
