#[cfg(test)]
mod tests {
    use abrash::particles::ParticleSystem;
    use abrash::texture::Texture;

    #[test]
    fn test_particle_dos_allocation() {
        let texture = Texture::new(2, 2).unwrap();
        // Initialize with small capacity
        let mut sys = ParticleSystem::new(10, texture);

        // malicious input: huge emission rate
        sys.emission_rate = 1_000_000_000.0;

        // Simulation of a frame spike (e.g. 1 second)
        // This should try to emit 1 billion particles
        sys.update(1.0);

        // If the system is secure, it should have capped the particles
        // If vulnerable, this might OOM or have 1 billion particles
        assert!(
            sys.particles.len() < 1_000_000,
            "Particle count explosion: {}",
            sys.particles.len()
        );
    }

    #[test]
    fn test_particle_dos_infinite_loop() {
        let texture = Texture::new(2, 2).unwrap();
        let mut sys = ParticleSystem::new(10, texture);

        // malicious input: huge dt
        sys.emission_rate = 10.0;

        // 100 years of lag
        // This should cause the while loop to run ~3e10 times
        let huge_dt = 3600.0 * 24.0 * 365.0 * 100.0;

        // This call will hang if not fixed
        // We can't easily timeout in this test harness, but if it runs, it passes.
        // Ideally we'd run this in a separate thread with timeout, but for now we trust the fix.
        // For reproduction, we can use a smaller but still large number that would be noticeable.
        sys.update(huge_dt);
    }
}
