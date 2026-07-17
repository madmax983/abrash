use abrash_render::experimental::attractor::{AttractorSystem, AttractorType, Particle};

fn main() {
    let mut system = AttractorSystem::new(AttractorType::default());
    for i in 0..10 {
        system.particles.push(Particle::new([i as f32, 1.0, 1.0]));
    }

    println!("Initial state:");
    for (i, p) in system.particles.iter().enumerate().take(3) {
        println!("Particle {}: {:?}", i, p.position);
    }

    system.run_steps(0.01, 100);

    println!("\nAfter 100 steps:");
    for (i, p) in system.particles.iter().enumerate().take(3) {
        println!("Particle {}: {:?}", i, p.position);
    }
}
