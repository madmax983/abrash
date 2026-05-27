fn main() {
    let min_z = 1.0f32;
    let max_z = 5.0f32;
    let min_z_bits = min_z.to_bits();
    let max_z_bits = max_z.to_bits();
    let range_bits = max_z_bits - min_z_bits;

    let scale_bits = (1023_u64 << 32) / range_bits as u64;

    for v in [1.0f32, 2.0f32, 3.0f32, 4.0f32, 5.0f32].iter() {
        let diff = v.to_bits().saturating_sub(min_z_bits);
        let t = ((diff as u64 * scale_bits) >> 32) as u32;
        println!("{}: t = {}", v, t);
    }
}
