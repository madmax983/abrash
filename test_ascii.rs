fn main() {
    let lum = 255;
    let char_idx = (lum as usize * 10) / 256;
    println!("char_idx for 255: {}", char_idx);
    let char_idx = (0 as usize * 10) / 256;
    println!("char_idx for 0: {}", char_idx);
    let char_idx = (128 as usize * 10) / 256;
    println!("char_idx for 128: {}", char_idx);
}
