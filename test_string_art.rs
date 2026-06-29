use std::f32::consts::PI;

fn main() {
    let num_pins = 200;
    let width = 800;
    let height = 800;
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = (cx.min(cy) - 2.0).max(1.0);

    let mut pins = Vec::with_capacity(num_pins);
    for i in 0..num_pins {
        let angle = (i as f32 / num_pins as f32) * 2.0 * PI;
        let x = cx + angle.cos() * radius;
        let y = cy + angle.sin() * radius;
        pins.push((x as i32, y as i32));
    }
    println!("Generated {} pins.", pins.len());
}
