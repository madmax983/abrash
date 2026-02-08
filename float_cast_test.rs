fn main() {
    let large_float = 1e30f32;
    let i = large_float as i32;
    println!("1e30 as i32 = {}", i);

    let nan_float = f32::NAN;
    let j = nan_float as i32;
    println!("NaN as i32 = {}", j);
}
