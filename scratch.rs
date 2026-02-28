use rayon::prelude::*;

fn main() {
    let items = vec![1, 2, 3];
    let result: Vec<_> = items
        .par_iter()
        .flat_map_iter(|&x| vec![x, x * 2])
        .collect();
    println!("{:?}", result);
}
