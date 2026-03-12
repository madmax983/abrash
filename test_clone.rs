use std::time::Instant;

fn main() {
    let mut boids: Vec<[f32; 6]> = vec![[0.0; 6]; 10000];

    let start = Instant::now();
    for _ in 0..100 {
        let old_boids = boids.clone();
        for i in 0..boids.len() {
            boids[i][0] += old_boids[i][0];
        }
    }
    println!("clone took: {:?}", start.elapsed());

    let start = Instant::now();
    let mut old_boids = vec![[0.0; 6]; 10000];
    for _ in 0..100 {
        old_boids.copy_from_slice(&boids);
        for i in 0..boids.len() {
            boids[i][0] += old_boids[i][0];
        }
    }
    println!("copy_from_slice took: {:?}", start.elapsed());
}
