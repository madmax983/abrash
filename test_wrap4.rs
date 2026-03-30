fn main() {
    let len = 4096;
    let tex_w = 32768; // max typical texture size
    let u_fix = 0;

    // We want u_max_64 <= i32::MAX but (u_max_64 >> 16) < tex_w
    // AND we want the 32-bit addition `u_fix = u_fix.wrapping_add(du_fix)` to wrap around
    // to a negative value.

    // BUT u_start_64 + du_64 * (len - 1) <= i32::MAX
    // Since du_64 >= 0 and len >= 1, du_64 * (len - 1) <= i32::MAX
    // If len = 4096, du_64 <= i32::MAX / 4095
    // du_64 <= 2147483647 / 4095 = 524416

    // If du_64 <= 524416, then wrapping_add(524416) will never wrap around past i32::MAX
    // within 4096 steps!
    // Because 524416 * 4096 = 2147983360
    // Wait... 524416 * 4096 = 2148007936
    // i32::MAX = 2147483647
    // IT CAN WRAP!

    let du_fix = 524416;
    let mut current_u_fix = 2147400000_i32; // close to MAX

    // but u_min_64 must be >= 0 and u_max_64 <= i32::MAX
    let u_start_64 = i64::from(current_u_fix);
    let u_end_64 = u_start_64 + i64::from(du_fix) * i64::from(len - 1);

    println!("start: {}, end: {}, max: {}", u_start_64, u_end_64, i32::MAX);
}
