fn main() {
    let mut x: i32 = 0;
    let mut y: i32 = -1;
    let z: i64 = i64::from(x) + i64::from(y);
    println!("z: {}", z);
}
