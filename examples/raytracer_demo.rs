#[cfg(feature = "nova")]
use abrash::experimental::raytracer::{Material, RayTracer, Scene, Sphere};
#[cfg(feature = "nova")]
use abrash::math::Vec3;
#[cfg(feature = "nova")]
use abrash::platform::PlatformContext;

#[cfg(feature = "nova")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width = 400;
    let height = 300;
    let mut ctx = PlatformContext::new("Ray Tracer", width, height)?;

    let material_ground = Material::lambertian(Vec3::new(0.8, 0.8, 0.0));
    let material_center = Material::lambertian(Vec3::new(0.1, 0.2, 0.5));
    let material_left = Material::metal(Vec3::new(0.8, 0.8, 0.8), 0.3);
    let material_right = Material::metal(Vec3::new(0.8, 0.6, 0.2), 1.0);

    let mut scene = Scene { objects: vec![] };
    scene.objects.push(Sphere {
        center: Vec3::new(0.0, -100.5, -1.0),
        radius: 100.0,
        material: material_ground,
    });
    scene.objects.push(Sphere {
        center: Vec3::new(0.0, 0.0, -1.0),
        radius: 0.5,
        material: material_center,
    });
    scene.objects.push(Sphere {
        center: Vec3::new(-1.0, 0.0, -1.0),
        radius: 0.5,
        material: material_left,
    });
    scene.objects.push(Sphere {
        center: Vec3::new(1.0, 0.0, -1.0),
        radius: 0.5,
        material: material_right,
    });

    let mut tracer = RayTracer::new(width, height);

    // Initial render
    tracer.render(&scene);

    let mut samples = 1;
    let max_samples = 100;

    while ctx.update() {
        if samples < max_samples {
            // Progressive rendering: add another sample per pixel and accumulate
            // For simplicity in this demo, we just re-render to show it works,
            // A real progressive renderer would accumulate samples in a float buffer.
            tracer.render(&scene);
            samples += 1;
            ctx.set_title(&format!("Ray Tracer - Samples: {}", samples));
        }

        ctx.draw_framebuffer(&tracer.framebuffer)?;
    }

    Ok(())
}

#[cfg(not(feature = "nova"))]
fn main() {}
