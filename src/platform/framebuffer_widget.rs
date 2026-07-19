//! Shared half-block (`▀`) framebuffer widget for terminal rendering.
//!
//! Used by both the TUI (crossterm) and WASM (ratzilla) backends to render
//! a [`Framebuffer`] into a ratatui terminal using Unicode half-block characters.

use crate::framebuffer::Framebuffer;
use ratatui::{buffer::Buffer, layout::Rect, style::Color, widgets::Widget};

/// Half-block framebuffer widget for terminal rendering.
///
/// Renders a [`Framebuffer`] using the Unicode upper half-block character (`▀`).
/// Each terminal cell represents two vertical pixels: the foreground color is the
/// top pixel, the background color is the bottom pixel.
pub struct FramebufferWidget<'a> {
    /// The framebuffer surface memory source to be blitted to the terminal screen.
    pub framebuffer: &'a Framebuffer,
}

impl Widget for FramebufferWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let term_w = area.width as usize;
        let term_h = area.height as usize;
        let fb_w = self.framebuffer.width() as usize;
        let fb_h = self.framebuffer.height() as usize;

        for y in 0..term_h {
            for x in 0..term_w {
                // Map terminal cell (x,y) to framebuffer coordinates
                // Nearest neighbor scaling
                let fb_x = (x * fb_w) / term_w;

                // Top sub-pixel
                let fb_y_top = (y * 2 * fb_h) / (term_h * 2);
                // Bottom sub-pixel
                let fb_y_bot = ((y * 2 + 1) * fb_h) / (term_h * 2);

                if fb_x >= fb_w || fb_y_top >= fb_h {
                    continue;
                }

                // Get colors
                let p_top = self
                    .framebuffer
                    .get_pixel(fb_x as i32, fb_y_top as i32)
                    .unwrap_or(0);
                let p_bot = self
                    .framebuffer
                    .get_pixel(fb_x as i32, fb_y_bot as i32)
                    .unwrap_or(0);

                // unpack (r, g, b) from u32 0xRRGGBB
                let (r1, g1, b1) = ((p_top >> 16) as u8, (p_top >> 8) as u8, p_top as u8);
                let (r2, g2, b2) = ((p_bot >> 16) as u8, (p_bot >> 8) as u8, p_bot as u8);

                if let Some(cell) = buf.cell_mut((area.x + x as u16, area.y + y as u16)) {
                    cell.set_char('▀')
                        .set_fg(Color::Rgb(r1, g1, b1))
                        .set_bg(Color::Rgb(r2, g2, b2));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_size_area_is_noop() {
        let fb = Framebuffer::new(10, 10).unwrap();
        let widget = FramebufferWidget { framebuffer: &fb };
        let area = Rect::new(0, 0, 0, 0);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 10));
        widget.render(area, &mut buf);
        // No panic = success
    }

    #[test]
    fn renders_known_color_pattern() {
        let mut fb = Framebuffer::new(2, 2).unwrap();
        // Set all pixels to red (0xRRGGBB)
        fb.set_pixel(0, 0, 0x00FF_0000);
        fb.set_pixel(1, 0, 0x00FF_0000);
        fb.set_pixel(0, 1, 0x0000_FF00);
        fb.set_pixel(1, 1, 0x0000_FF00);

        let widget = FramebufferWidget { framebuffer: &fb };
        let area = Rect::new(0, 0, 2, 1); // 2 wide, 1 tall = 2 pixels vertically via half-block
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        widget.render(area, &mut buf);

        // Top pixel = red (fg), bottom pixel = green (bg)
        let cell = buf.cell((0, 0)).unwrap();
        assert_eq!(cell.symbol(), "▀");
        assert_eq!(cell.fg, Color::Rgb(255, 0, 0));
        assert_eq!(cell.bg, Color::Rgb(0, 255, 0));
    }

    #[test]
    fn color_unpacking_pure_red() {
        let mut fb = Framebuffer::new(1, 2).unwrap();
        fb.set_pixel(0, 0, 0x00FF_0000);
        fb.set_pixel(0, 1, 0x00FF_0000);

        let widget = FramebufferWidget { framebuffer: &fb };
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        widget.render(area, &mut buf);

        let cell = buf.cell((0, 0)).unwrap();
        assert_eq!(cell.fg, Color::Rgb(255, 0, 0));
        assert_eq!(cell.bg, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn color_unpacking_pure_green() {
        let mut fb = Framebuffer::new(1, 2).unwrap();
        fb.set_pixel(0, 0, 0x0000_FF00);
        fb.set_pixel(0, 1, 0x0000_FF00);

        let widget = FramebufferWidget { framebuffer: &fb };
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        widget.render(area, &mut buf);

        let cell = buf.cell((0, 0)).unwrap();
        assert_eq!(cell.fg, Color::Rgb(0, 255, 0));
        assert_eq!(cell.bg, Color::Rgb(0, 255, 0));
    }

    #[test]
    fn color_unpacking_pure_blue() {
        let mut fb = Framebuffer::new(1, 2).unwrap();
        fb.set_pixel(0, 0, 0x0000_00FF);
        fb.set_pixel(0, 1, 0x0000_00FF);

        let widget = FramebufferWidget { framebuffer: &fb };
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        widget.render(area, &mut buf);

        let cell = buf.cell((0, 0)).unwrap();
        assert_eq!(cell.fg, Color::Rgb(0, 0, 255));
        assert_eq!(cell.bg, Color::Rgb(0, 0, 255));
    }

    #[test]
    fn color_unpacking_arbitrary_value() {
        // 0xABCDEF => R=0xAB(171), G=0xCD(205), B=0xEF(239)
        let mut fb = Framebuffer::new(1, 2).unwrap();
        fb.set_pixel(0, 0, 0x00AB_CDEF);
        fb.set_pixel(0, 1, 0x00AB_CDEF);

        let widget = FramebufferWidget { framebuffer: &fb };
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        widget.render(area, &mut buf);

        let cell = buf.cell((0, 0)).unwrap();
        assert_eq!(cell.fg, Color::Rgb(0xAB, 0xCD, 0xEF));
        assert_eq!(cell.bg, Color::Rgb(0xAB, 0xCD, 0xEF));
    }

    #[test]
    fn scaling_large_framebuffer_to_small_area() {
        // 100x100 framebuffer rendered into 10x5 terminal area
        let mut fb = Framebuffer::new(100, 100).unwrap();
        // Fill entire framebuffer with a known color
        fb.clear(0x0033_6699);

        let widget = FramebufferWidget { framebuffer: &fb };
        let area = Rect::new(0, 0, 10, 5);
        let mut buf = Buffer::empty(Rect::new(0, 0, 10, 5));
        widget.render(area, &mut buf);

        // Every cell should be filled with half-block characters
        for y in 0..5u16 {
            for x in 0..10u16 {
                let cell = buf.cell((x, y)).unwrap();
                assert_eq!(
                    cell.symbol(),
                    "▀",
                    "Cell ({x},{y}) should have half-block char"
                );
                assert_eq!(cell.fg, Color::Rgb(0x33, 0x66, 0x99));
                assert_eq!(cell.bg, Color::Rgb(0x33, 0x66, 0x99));
            }
        }
    }

    #[test]
    fn single_pixel_framebuffer() {
        // 1x1 framebuffer rendered into a 1x1 area
        let mut fb = Framebuffer::new(1, 1).unwrap();
        fb.set_pixel(0, 0, 0x00DE_AD42);

        let widget = FramebufferWidget { framebuffer: &fb };
        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        widget.render(area, &mut buf);

        let cell = buf.cell((0, 0)).unwrap();
        assert_eq!(cell.symbol(), "▀");
        // Top pixel maps to (0,0), bottom pixel also maps to (0,0) for a 1x1 fb
        assert_eq!(cell.fg, Color::Rgb(0xDE, 0xAD, 0x42));
        assert_eq!(cell.bg, Color::Rgb(0xDE, 0xAD, 0x42));
    }
}
