use abrash_render::experimental::doom_fire::DoomFire;
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowBuilder;

const SCALE: u32 = 2;
const FIRE_WIDTH: u32 = 320;
const FIRE_HEIGHT: u32 = 168;

fn main() {
    let tui = std::env::args().any(|arg| arg == "--tui");

    if tui {
        run_tui();
    } else {
        run_winit();
    }
}

#[cfg(feature = "backend-tui")]
fn run_tui() {
    use abrash::backend_tui::TuiRunner;
    let mut fire = DoomFire::new();
    TuiRunner::run(
        FIRE_WIDTH,
        FIRE_HEIGHT,
        "DOOM Fire (Space: Toggle Fire, Esc: Exit)",
        move |fb| {
            fire.update();
            fb.as_mut_slice().copy_from_slice(fire.buffer.as_slice());
        },
    )
    .unwrap();
}

#[cfg(not(feature = "backend-tui"))]
fn run_tui() {
    println!("TUI backend not enabled. Run with `--features backend-tui`.");
}

fn run_winit() {
    let event_loop = EventLoop::new().unwrap();
    let window = Rc::new(
        WindowBuilder::new()
            .with_title("DOOM Fire - Space to toggle, Esc to exit")
            .with_inner_size(winit::dpi::LogicalSize::new(
                FIRE_WIDTH * SCALE,
                FIRE_HEIGHT * SCALE,
            ))
            .build(&event_loop)
            .unwrap(),
    );

    let context = Context::new(window.clone()).unwrap();
    let mut surface = Surface::new(&context, window.clone()).unwrap();

    let mut fire = DoomFire::new();

    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                elwt.exit();
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(key_code),
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    },
                ..
            } => match key_code {
                KeyCode::Escape => elwt.exit(),
                KeyCode::Space => {
                    fire.active = !fire.active;
                }
                _ => {}
            },
            Event::AboutToWait => {
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                let size = window.inner_size();
                if size.width == 0 || size.height == 0 {
                    return;
                }

                surface
                    .resize(
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    )
                    .unwrap();

                fire.update();

                let mut buffer = surface.buffer_mut().unwrap();

                // Simple nearest-neighbor upscaling
                for y in 0..size.height {
                    let src_y = (y * FIRE_HEIGHT) / size.height;
                    for x in 0..size.width {
                        let src_x = (x * FIRE_WIDTH) / size.width;
                        let src_idx = (src_y * FIRE_WIDTH + src_x) as usize;
                        let dst_idx = (y * size.width + x) as usize;
                        buffer[dst_idx] = fire.buffer.as_slice()[src_idx];
                    }
                }

                buffer.present().unwrap();
            }
            _ => {}
        })
        .unwrap();
}
