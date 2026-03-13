use abrash::framebuffer::Framebuffer;
use abrash::math::Mat4;
use abrash::post_process::{BloomConfig, apply_bloom, apply_sobel, apply_ssao};
use abrash::zbuffer::ZBuffer;
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn test_post_process_concurrency() {
    let thread_count = 8;
    let iterations = 50;

    // Barrier to synchronize start (maximize contention)
    let barrier = Arc::new(Barrier::new(thread_count));

    let mut handles = vec![];

    for t_id in 0..thread_count {
        let b = barrier.clone();
        handles.push(thread::spawn(move || {
            b.wait();

            for i in 0..iterations {
                // Randomize size slightly to stress reallocation
                // 100x100 base, + up to 20
                let w = 100 + (i % 20) as u32;
                let h = 100 + ((i + t_id) % 20) as u32;

                let mut fb = Framebuffer::new(w, h).unwrap();
                let zb = ZBuffer::new(w, h).unwrap();

                // Fill with garbage
                fb.clear(0xFF00FFFF);

                // Call Bloom
                // threshold, blur_radius, intensity
                let config = BloomConfig {
                    threshold: 200,
                    blur_radius: 5,
                    intensity: 1.5,
                };
                apply_bloom(&mut fb, &config);

                // Call SSAO
                let proj = Mat4::identity();
                apply_ssao(
                    &mut fb,
                    &zb,
                    &proj,
                    &abrash::post_process::SsaoConfig {
                        radius: 0.5,
                        bias: 0.025,
                        intensity: 2.0,
                    },
                );

                // Call Sobel
                apply_sobel(&mut fb);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}
