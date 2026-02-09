fn main() {
    let mut tuple = (vec![1], vec![2]);
    let reference = &mut tuple;
    let (a, b) = reference;
    println!("{:?} {:?}", a, b);
}
