fn fast_atan2(y: f32, x: f32) -> f32 {
    let abs_y = y.abs() + 1e-10;
    let abs_x = x.abs() + 1e-10;
    let r = (abs_x - abs_y) / (abs_x + abs_y);
    let mut angle = std::f32::consts::FRAC_PI_4 - std::f32::consts::FRAC_PI_4 * r;
    if x < 0.0 {
        angle = std::f32::consts::PI - angle;
    }
    if y < 0.0 {
        angle = -angle;
    }
    angle
}

fn main() {
    let test_cases: Vec<(f32, f32)> = vec![
        (0.0, 1.0),
        (1.0, 1.0),
        (1.0, 0.0),
        (1.0, -1.0),
        (0.0, -1.0),
        (-1.0, -1.0),
        (-1.0, 0.0),
        (-1.0, 1.0),
    ];

    for (y, x) in test_cases {
        let std_res = y.atan2(x);
        let fast_res = fast_atan2(y, x);
        println!("y: {}, x: {}, std: {:.4}, fast: {:.4}, diff: {:.4}", y, x, std_res, fast_res, (std_res - fast_res).abs());
    }
}
