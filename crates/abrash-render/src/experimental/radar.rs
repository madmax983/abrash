use abrash_core::framebuffer::Framebuffer;
use abrash_core::zbuffer::ZBuffer;
use abrash_core::math::fast_atan2;
use std::f32::consts::TAU;

pub struct RadarConfig {
    pub center_x: f32,
    pub center_y: f32,
    pub angle: f32,
    pub sweep_width: f32,
    pub grid_color: u32,
    pub sweep_color: u32,
    pub blip_color: u32,
    pub bg_color: u32,
    pub ring_spacing: f32,
}

pub fn apply_radar(fb: &mut Framebuffer, zb: &ZBuffer, config: &RadarConfig) {
    let width = fb.width() as usize;
    let buffer = fb.as_mut_slice();
    let z_buffer = zb.as_slice();

    let cx = config.center_x;
    let cy = config.center_y;

    for (y, row) in buffer.chunks_exact_mut(width).enumerate() {
        let z_row = &z_buffer[y * width..(y + 1) * width];
        let dy = y as f32 - cy;

        for (x, pixel) in row.iter_mut().enumerate() {
            let dx = x as f32 - cx;
            let dist_sq = dx * dx + dy * dy;
            let dist = dist_sq.sqrt();

            let mut out_color = config.bg_color;

            if dist % config.ring_spacing < 1.5 || dx.abs() < 1.0 || dy.abs() < 1.0 {
                out_color = config.grid_color;
            }

            let mut pixel_angle = fast_atan2(dy, dx);
            if pixel_angle < 0.0 {
                pixel_angle += TAU;
            }

            let mut angle_diff = config.angle - pixel_angle;
            if angle_diff < 0.0 {
                angle_diff += TAU;
            }

            if angle_diff < config.sweep_width {
                let z_val = z_row[x];
                if z_val < f32::INFINITY {
                    out_color = config.blip_color;
                } else {
                    out_color = config.sweep_color;
                }
            }

            *pixel = out_color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abrash_core::framebuffer::Framebuffer;
    use abrash_core::zbuffer::ZBuffer;

    #[test]
    fn test_apply_radar() {
        let mut fb = Framebuffer::new(100, 100).unwrap();
        let zb = ZBuffer::new(100, 100).unwrap();
        let config = RadarConfig {
            center_x: 50.0,
            center_y: 50.0,
            angle: 0.0,
            sweep_width: 0.2,
            grid_color: 0x0000FF00,
            sweep_color: 0x0000FF00,
            blip_color: 0x00FFFFFF,
            bg_color: 0xFF000000,
            ring_spacing: 20.0,
        };
        apply_radar(&mut fb, &zb, &config);
        assert_eq!(fb.as_slice()[0], 0xFF000000);
    }
}
