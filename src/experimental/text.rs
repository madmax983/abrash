//! Simple 5x7 bitmap font rendering.

use crate::framebuffer::Framebuffer;

/// 5x7 Bitmap Font packed into u64.
/// Each character is 5 bits wide * 7 rows = 35 bits.
/// Row 0 (top) is bits 0-4.
pub const FONT: [u64; 128] = [
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // '\t' (Empty)
    0x0,                   // '\n' (Empty)
    0x0,                   // '\x0b' (Empty)
    0x0,                   // '\x0c' (Empty)
    0x0,                   // '\r' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0,                   // ' ' (Empty)
    0x0000_0000_0000_0000, // ' '
    0x0000_0001_0042_1084, // '!'
    0x0,                   // '"' (Empty)
    0x0,                   // '#' (Empty)
    0x0,                   // '$' (Empty)
    0x0,                   // '%' (Empty)
    0x0,                   // '&' (Empty)
    0x0,                   // ''' (Empty)
    0x0000_0001_0410_8444, // '('
    0x0000_0001_1108_4104, // ')'
    0x0000_0000_22a2_2a20, // '*'
    0x0000_0000_084f_9080, // '+'
    0x0,                   // ',' (Empty)
    0x0000_0000_000f_8000, // '-'
    0x0000_0001_8c00_0000, // '.'
    0x0000_0000_0222_2200, // '/'
    0x0000_0003_a319_d72e, // '0'
    0x0000_0003_8842_10c4, // '1'
    0x0000_0007_c444_422e, // '2'
    0x0000_0003_a306_221f, // '3'
    0x0000_0002_11f4_a988, // '4'
    0x0000_0003_a308_3c3f, // '5'
    0x0000_0003_a317_842e, // '6'
    0x0000_0000_4222_221f, // '7'
    0x0000_0003_a317_462e, // '8'
    0x0000_0003_910f_462e, // '9'
    0x0000_0000_0c60_18c0, // ':'
    0x0,                   // ';' (Empty)
    0x0000_0000_1820_8980, // '<'
    0x0000_0000_01f0_7c00, // '='
    0x0000_0000_0c88_20c0, // '>'
    0x0000_0001_0044_422e, // '?'
    0x0,                   // '@' (Empty)
    0x0000_0004_631f_c62e, // 'A'
    0x0000_0003_e317_c62f, // 'B'
    0x0000_0003_a210_862e, // 'C'
    0x0000_0003_e318_c62f, // 'D'
    0x0000_0007_c217_843f, // 'E'
    0x0000_0000_4217_843f, // 'F'
    0x0000_0003_a31c_842e, // 'G'
    0x0000_0004_631f_c631, // 'H'
    0x0000_0003_8842_108e, // 'I'
    0x0000_0003_a308_421c, // 'J'
    0x0000_0004_5251_9531, // 'K'
    0x0000_0007_c210_8421, // 'L'
    0x0000_0004_6318_d771, // 'M'
    0x0000_0004_631c_d671, // 'N'
    0x0000_0003_a318_c62e, // 'O'
    0x0000_0000_4217_c62f, // 'P'
    0x0000_0005_9358_c62e, // 'Q'
    0x0000_0004_5257_c62f, // 'R'
    0x0000_0003_a307_062e, // 'S'
    0x0000_0001_0842_109f, // 'T'
    0x0000_0003_a318_c631, // 'U'
    0x0000_0001_1518_c631, // 'V'
    0x0000_0004_7758_c631, // 'W'
    0x0000_0004_62a2_2a31, // 'X'
    0x0000_0001_0842_2a31, // 'Y'
    0x0000_0007_c222_221f, // 'Z'
    0x0000_0003_8421_084e, // '['
    0x0,                   // '\' (Empty)
    0x0000_0003_9084_210e, // ']'
    0x0,                   // '^' (Empty)
    0x0,                   // '_' (Empty)
    0x0,                   // '`' (Empty)
    0x0,                   // 'a' (Empty)
    0x0,                   // 'b' (Empty)
    0x0,                   // 'c' (Empty)
    0x0,                   // 'd' (Empty)
    0x0,                   // 'e' (Empty)
    0x0,                   // 'f' (Empty)
    0x0,                   // 'g' (Empty)
    0x0,                   // 'h' (Empty)
    0x0,                   // 'i' (Empty)
    0x0,                   // 'j' (Empty)
    0x0,                   // 'k' (Empty)
    0x0,                   // 'l' (Empty)
    0x0,                   // 'm' (Empty)
    0x0,                   // 'n' (Empty)
    0x0,                   // 'o' (Empty)
    0x0,                   // 'p' (Empty)
    0x0,                   // 'q' (Empty)
    0x0,                   // 'r' (Empty)
    0x0,                   // 's' (Empty)
    0x0,                   // 't' (Empty)
    0x0,                   // 'u' (Empty)
    0x0,                   // 'v' (Empty)
    0x0,                   // 'w' (Empty)
    0x0,                   // 'x' (Empty)
    0x0,                   // 'y' (Empty)
    0x0,                   // 'z' (Empty)
    0x0,                   // '{' (Empty)
    0x0,                   // '|' (Empty)
    0x0,                   // '}' (Empty)
    0x0,                   // '~' (Empty)
    0x0,                   // ' ' (Empty)
];

/// Draws a single character to the framebuffer.
///
/// * `x`, `y` - Top-left coordinate.
/// * `c` - The character to draw.
/// * `color` - The color (0xAARRGGBB).
/// * `scale` - Integer scaling factor (e.g., 1, 2, 3).
pub fn draw_char(fb: &mut Framebuffer, x: i32, y: i32, c: char, color: u32, scale: i32) {
    if scale <= 0 {
        return;
    }

    let char_idx = c as usize;
    if char_idx >= FONT.len() {
        return;
    }

    let bitmap = FONT[char_idx];
    if bitmap == 0 && c != ' ' {
        // Fallback for missing characters? Or just skip.
        // For now, skip.
        return;
    }

    // Unpack bits
    for row in 0..7 {
        for col in 0..5 {
            let bit_idx = row * 5 + col;
            if (bitmap >> bit_idx) & 1 == 1 {
                // Draw pixel(s) based on scale
                for sy in 0..scale {
                    for sx in 0..scale {
                        fb.set_pixel(x + col * scale + sx, y + row * scale + sy, color);
                    }
                }
            }
        }
    }
}

/// Draws a string of text to the framebuffer.
///
/// * `x`, `y` - Top-left coordinate.
/// * `text` - The string to draw.
/// * `color` - The color (0xAARRGGBB).
/// * `scale` - Integer scaling factor.
pub fn draw_text(fb: &mut Framebuffer, x: i32, y: i32, text: &str, color: u32, scale: i32) {
    let mut cursor_x = x;
    let mut cursor_y = y;
    // Char width 5, spacing 1 = 6 pixels total width per char (at scale 1)
    let stride = 6 * scale;
    let line_height = 8 * scale; // 7 height + 1 spacing

    for c in text.chars() {
        if c == '\n' {
            cursor_x = x;
            cursor_y += line_height;
            continue;
        }
        draw_char(fb, cursor_x, cursor_y, c, color, scale);
        cursor_x += stride;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_char() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        // Draw 'I' (simple) at 0,0
        // 'I' bitmap:
        //  XXX
        //   X
        //   X
        //   X
        //   X
        //   X
        //  XXX
        draw_char(&mut fb, 0, 0, 'I', 0xFFFFFFFF, 1);

        // Check top-left pixel (0,0) -> should be ' ' (empty) in 5x7 box for 'I'
        // Wait, 'I' definition:
        // " XXX ", -> row 0
        // Row 0, col 0 is ' '. col 1 is 'X'.

        // Row 0, col 1 should be white.
        assert_eq!(fb.get_pixel(1, 0), Some(0xFFFFFFFF));
        // Row 0, col 0 should be black (transparent/unwritten -> 0 is default clear color if cleared, but new framebuffer is black)
        // Framebuffer::new inits to Black (0xFF000000).
        // Wait, Framebuffer::new inits to 0xFF000000 (Black Opaque).
        assert_eq!(fb.get_pixel(0, 0), Some(0xFF000000));
    }

    #[test]
    fn test_draw_text_spacing() {
        let mut fb = Framebuffer::new(20, 10).unwrap();
        draw_text(&mut fb, 0, 0, "AB", 0xFFFFFFFF, 1);

        // 'A' width 5. Spacing 1. 'B' starts at x=6.
        // Check pixel at x=6 (start of B).
        // 'B' row 0 is "XXXX ".
        // So (6, 0) should be set.
        assert_eq!(fb.get_pixel(6, 0), Some(0xFFFFFFFF));
    }
}
