with open("src/experimental/boids.rs", "r") as f:
    content = f.read()

# We need to add the closing braces for `OLD_BOIDS.with(|old_boids_cell| {`
# Where does it end?
old = """            // Update position
            boid.position.x += boid.velocity.x * delta_time;
            boid.position.y += boid.velocity.y * delta_time;
            boid.position.z += boid.velocity.z * delta_time;
        });
    }"""

new = """            // Update position
            boid.position.x += boid.velocity.x * delta_time;
            boid.position.y += boid.velocity.y * delta_time;
            boid.position.z += boid.velocity.z * delta_time;
        });
        }); // Close OLD_BOIDS.with
    }"""

if old in content:
    with open("src/experimental/boids.rs", "w") as f:
        f.write(content.replace(old, new))
    print("Fixed.")
else:
    print("Not found.")
