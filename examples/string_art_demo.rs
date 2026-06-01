use abrash_core::framebuffer::Framebuffer;
use abrash_render::experimental::string_art::StringArtGenerator;

fn main() {
    let width = 400;
    let height = 400;
    let mut target = Framebuffer::new(width, height).unwrap();
    target.clear(0xFFFFFFFF); // White background

    // Draw a dark circle in the middle as target
    for y in 100..300 {
        for x in 100..300 {
            let dx = x as f32 - 200.0;
            let dy = y as f32 - 200.0;
            if dx * dx + dy * dy < 100.0 * 100.0 {
                target.set_pixel(x, y, 0xFF00_0000); // Black
            }
        }
    }

    println!("Generating string art...");
    let num_pegs = 288;
    let num_lines = 1000;
    let mut generator = StringArtGenerator::new(target, num_pegs, num_lines);
    let path = generator.generate();

    println!("Generated path of length {}", path.len());

    // Render it back
    let mut result = Framebuffer::new(width, height).unwrap();
    result.clear(0xFFFFFFFF); // White background

    // Reconstruct pegs to render lines
    let radius = width.min(height) as f32 / 2.0 - 2.0;
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let mut pegs = Vec::with_capacity(num_pegs);
    for i in 0..num_pegs {
        let angle = i as f32 * 2.0 * std::f32::consts::PI / num_pegs as f32;
        let x = cx + radius * angle.cos();
        let y = cy + radius * angle.sin();
        pegs.push((x as i32, y as i32));
    }

    let mut current_peg = path[0];
    for &next_peg in path.iter().skip(1) {
        let p0 = pegs[current_peg];
        let p1 = pegs[next_peg];
        draw_line(&mut result, p0.0, p0.1, p1.0, p1.1, 0xFF000000);
        current_peg = next_peg;
    }

    println!("Done. Run with winit backend or save to file (not implemented in this minimal demo).");
}

fn draw_line(fb: &mut Framebuffer, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: u32) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        fb.set_pixel(x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}
