with open("src/experimental/boids.rs", "r") as f:
    content = f.read()

old = """        OLD_BOIDS.with(|old_boids_cell| {
            let mut old_boids = old_boids_cell.borrow_mut();
            old_boids.clear();
            old_boids.extend(self.boids.iter().cloned());

            #[cfg(feature = "parallel")]
            let iter = self.boids.par_iter_mut();
            #[cfg(not(feature = "parallel"))]
            let iter = self.boids.iter_mut();

        iter.enumerate().for_each(|(i, boid)| {"""

new = """        OLD_BOIDS.with(|old_boids_cell| {
            let mut old_boids_mut = old_boids_cell.borrow_mut();
            old_boids_mut.clear();
            old_boids_mut.extend(self.boids.iter().cloned());

            // Extract the slice so we can safely pass it to the Rayon parallel closure
            let old_boids: &[Boid] = &old_boids_mut;

            #[cfg(feature = "parallel")]
            let iter = self.boids.par_iter_mut();
            #[cfg(not(feature = "parallel"))]
            let iter = self.boids.iter_mut();

        iter.enumerate().for_each(|(i, boid)| {"""

if old in content:
    with open("src/experimental/boids.rs", "w") as f:
        f.write(content.replace(old, new))
    print("Fixed.")
else:
    print("Not found.")
