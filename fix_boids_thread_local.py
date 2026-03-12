with open("src/experimental/boids.rs", "r") as f:
    content = f.read()

old = """    /// Updates the flock by one time step.
    pub fn update(&mut self, delta_time: f32) {
        let old_boids = self.boids.clone();

        #[cfg(feature = "parallel")]
        let iter = self.boids.par_iter_mut();
        #[cfg(not(feature = "parallel"))]
        let iter = self.boids.iter_mut();"""

new = """    /// Updates the flock by one time step.
    pub fn update(&mut self, delta_time: f32) {
        thread_local! {
            static OLD_BOIDS: std::cell::RefCell<Vec<Boid>> = const { std::cell::RefCell::new(Vec::new()) };
        }

        OLD_BOIDS.with(|old_boids_cell| {
            let mut old_boids = old_boids_cell.borrow_mut();
            old_boids.clear();
            old_boids.extend(self.boids.iter().cloned());

            #[cfg(feature = "parallel")]
            let iter = self.boids.par_iter_mut();
            #[cfg(not(feature = "parallel"))]
            let iter = self.boids.iter_mut();"""

if old in content:
    with open("src/experimental/boids.rs", "w") as f:
        f.write(content.replace(old, new))
    print("Fixed.")
else:
    print("Not found.")
