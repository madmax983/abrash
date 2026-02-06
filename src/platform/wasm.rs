//! WASM backend using ratzilla (ratatui for the web).
//!
//! Renders the framebuffer into a browser DOM element using the same
//! half-block widget shared with the TUI backend.

use super::Event;
use super::framebuffer_widget::FramebufferWidget;
use crate::framebuffer::Framebuffer;
use crate::zbuffer::ZBuffer;
use ratatui::Terminal;
use ratzilla::event::KeyCode;
use ratzilla::{DomBackend, WebRenderer};
use std::cell::RefCell;
use std::rc::Rc;

/// Run a WASM render loop using the ratzilla callback model.
///
/// `render_fn` is called each frame with mutable access to the framebuffer,
/// z-buffer, and any pending input events. The function is responsible for
/// clearing and drawing into the framebuffer; the WASM backend then blits the
/// result to the browser via the half-block `FramebufferWidget`.
pub fn run_wasm<F>(title: &str, width: u32, height: u32, mut render_fn: F)
where
    F: FnMut(&mut Framebuffer, &mut ZBuffer, &[Event]) + 'static,
{
    let backend = DomBackend::new().expect("failed to create DomBackend");
    let terminal = Terminal::new(backend).expect("failed to create Terminal");

    let framebuffer = Rc::new(RefCell::new(
        Framebuffer::new(width, height).expect("failed to create Framebuffer"),
    ));
    let zbuffer = Rc::new(RefCell::new(
        ZBuffer::new(width, height).expect("failed to create ZBuffer"),
    ));
    let events: Rc<RefCell<Vec<Event>>> = Rc::new(RefCell::new(Vec::new()));

    // Wire up key events
    terminal.on_key_event({
        let events = Rc::clone(&events);
        move |key_event| match key_event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                events.borrow_mut().push(Event::Close);
            }
            _ => {}
        }
    });

    // Render loop — ratzilla calls this on each animation frame
    let _title = title.to_string();
    terminal.draw_web(move |f| {
        // Drain pending events
        let pending: Vec<Event> = events.borrow_mut().drain(..).collect();

        // Let the application render into the framebuffer
        render_fn(
            &mut framebuffer.borrow_mut(),
            &mut zbuffer.borrow_mut(),
            &pending,
        );

        // Blit framebuffer to the terminal frame
        let widget = FramebufferWidget {
            framebuffer: &framebuffer.borrow(),
        };
        f.render_widget(widget, f.area());
    });
}
