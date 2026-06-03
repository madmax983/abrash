#!/bin/bash
patch -p1 << 'PATCH'
--- a/crates/abrash-render/src/experimental/boids.rs
+++ b/crates/abrash-render/src/experimental/boids.rs
@@ -227,7 +227,7 @@
                 boid.velocity.x *= f;
                 boid.velocity.y *= f;
                 boid.velocity.z *= f;
-            } else if speed_sq < min_speed_sq && speed_sq > 0.000001 {
+            } else if speed_sq < min_speed_sq && speed_sq > 0.000_001 {
                 let f = self.config.min_speed * abrash_core::math::fast_inv_sqrt(speed_sq);
                 boid.velocity.x *= f;
                 boid.velocity.y *= f;
PATCH
