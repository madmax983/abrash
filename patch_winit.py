import re

with open("src/platform/winit.rs", "r") as f:
    content = f.read()

# Let's write the exact code we want for the new handle_window_event function
# and replace the match event inside winit.rs.

new_func = """fn handle_window_event<A: WindowApp>(
    app: &mut A,
    event: &WindowEvent,
    window_for_loop: &Window,
    event_loop_target: &EventLoopWindowTarget<()>,
    clock: &mut FrameClock,
) -> Result<(), String> {
    match event {
        WindowEvent::CloseRequested => {
            event_loop_target.exit();
        }
        WindowEvent::Resized(size) => {
            if size.width != 0 && size.height != 0 {
                app.resize(
                    WindowContext {
                        event_loop: event_loop_target,
                        window: window_for_loop.clone().into(),
                        dt_seconds: 0.0,
                    },
                    size.width,
                    size.height,
                ).map_err(|e| e.to_string())?;
            }
        }
        WindowEvent::RedrawRequested => {
            let dt_seconds = clock.tick();
            let redraw_context = WindowContext {
                event_loop: event_loop_target,
                window: window_for_loop.clone().into(),
                dt_seconds,
            };
            app.update(redraw_context.clone()).map_err(|e| e.to_string())?;
            app.render(redraw_context).map_err(|e| e.to_string())?;
        }
        other => {
            let context = WindowContext {
                event_loop: event_loop_target,
                window: window_for_loop.clone().into(),
                dt_seconds: 0.0,
            };
            app.input(context, other).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
"""

# Wait, the `window_for_loop.clone().into()` might not be right depending on what window is. It's an `Arc<Window>`.
