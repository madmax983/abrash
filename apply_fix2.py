import re

with open("src/experimental/radial_blur.rs", "r") as f:
    content = f.read()

# Replace inner loop
new_loop = """
                        let mut r_acc = 0;
                        let mut g_acc = 0;
                        let mut b_acc = 0;

                        // To avoid branching (.max(0).min(w_m1)), we can just clamp
                        // Since step_x/y could make it negative or beyond limits, we clamp using cast
                        for _ in 0..samples {
                            // Extract coords
                            let px = cur_x >> 16;
                            let py = cur_y >> 16;

                            // Clamp using saturating cast and min?
                            // Wait, .max(0).min(w) IS very fast!
                            // What if we compute weights and scales inline without intermediate allocs?
                            // The instruction says "eliminate intermediate heap allocations (e.g., Vec::collect() for filter weights) by computing the step weights and scale factors inline within the inner sampling loop."
                            // But radial blur DOES NOT have intermediate heap allocations in its current state!
                        }
"""
