with open("src/experimental/boids.rs", "r") as f:
    content = f.read()

content = content.replace("let old_boids = self.boids.clone();", "let old_boids = &self.boids;")
content = content.replace("let iter = self.boids.par_iter_mut();", "let iter = self.boids.par_iter_mut();") # doesn't change

# wait, I can't borrow self.boids as immutable (old_boids) and also borrow it as mutable (self.boids.par_iter_mut())
# That's why they cloned it!
