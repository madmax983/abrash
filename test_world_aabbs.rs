use std::time::Instant;

fn main() {
    let num_objects = 10000;

    let mut world_aabbs = vec![0; 0];

    let start = Instant::now();
    for _ in 0..100 {
        world_aabbs.clear();
        world_aabbs.reserve(num_objects);
        for i in 0..num_objects {
            world_aabbs.push(i);
        }
    }
    println!("push took: {:?}", start.elapsed());

    let start = Instant::now();
    for _ in 0..100 {
        world_aabbs.clear();
        world_aabbs.reserve(num_objects);
        let uninit = world_aabbs.spare_capacity_mut();
        for i in 0..num_objects {
            uninit[i].write(i);
        }
        unsafe { world_aabbs.set_len(num_objects); }
    }
    println!("uninit write took: {:?}", start.elapsed());

    let start = Instant::now();
    for _ in 0..100 {
        world_aabbs.clear();
        world_aabbs.extend((0..num_objects).map(|i| i));
    }
    println!("extend took: {:?}", start.elapsed());
}
