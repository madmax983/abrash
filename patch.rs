pub fn clear_rect(width_buf: u32, height_buf: u32, pixels: &mut [u32], x: i32, y: i32, width: u32, height: u32, color: u32) {
    if width == 0 || height == 0 {
        return;
    }

    let end_x_u64 = x as i64 + width as i64;
    let end_y_u64 = y as i64 + height as i64;

    let start_x = x.clamp(0, width_buf as i32) as u32;
    let start_y = y.clamp(0, height_buf as i32) as u32;
    let end_x = end_x_u64.clamp(0, width_buf as i64) as u32;
    let end_y = end_y_u64.clamp(0, height_buf as i64) as u32;

    if start_x >= end_x || start_y >= end_y {
        return;
    }

    let buf_width = width_buf as usize;
    if start_x == 0 && end_x == width_buf {
        let start = start_y as usize * buf_width;
        let end = end_y as usize * buf_width;
        pixels[start..end].fill(color);
    } else {
        let start = start_x as usize;
        let len = (end_x - start_x) as usize;
        let start_idx = start_y as usize * buf_width;
        let end_idx = end_y as usize * buf_width;
        for row in pixels[start_idx..end_idx].chunks_exact_mut(buf_width) {
            row[start..start + len].fill(color);
        }
    }
}
