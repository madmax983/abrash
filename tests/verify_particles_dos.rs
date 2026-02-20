use abrash::particles::ParticleSystem;
use abrash::texture::Texture;

#[test]
fn test_particle_system_limits() {
    let texture = Texture::new(1, 1).unwrap();
    let max_particles = 10;
    let mut sys = ParticleSystem::new(max_particles, texture);

    sys.start_life = 100.0; // Ensure particles survive the update
    sys.emission_rate = 1000.0;
    sys.update(1.0); // Should try to emit 1000 particles

    assert!(sys.particles.len() <= max_particles, "Particle count {} exceeded max {}", sys.particles.len(), max_particles);
}
