//! Demonstration of the Nova Fog filter.

use abrash::experimental::fog::{apply_fog, FogConfig};
use abrash::framebuffer::Framebuffer;
use abrash::zbuffer::ZBuffer;
use abrash::platform::{Event, Window};
use abrash::math::{Mat4, Vec3};
use abrash::scene::{Camera, Scene, SceneObject};
use abrash::mesh::Mesh;
use std::f32::consts::PI;
use std::sync::Arc;

fn main() {
    let mut window = Window::new("Depth Fog Demo (Nova)", 800, 600).unwrap();

    let width = 800;
    let height = 600;

    let mut fb = Framebuffer::new(width, height).unwrap();
    let mut zb = ZBuffer::new(width, height).unwrap();

    let mut angle: f32 = 0.0;

    // Setup camera
    let projection = Mat4::perspective(PI / 3.0, width as f32 / height as f32, 0.1, 100.0);
    let camera = Camera::new(Mat4::identity(), projection);
    let mut scene = Scene::new(camera);

    // Add some geometry to the scene
    let cube_mesh = Arc::new(Mesh::cube(1.0));

    // Add floor
    let floor_transform = Mat4::translation(0.0, -2.0, 0.0) * Mat4::scale(20.0, 0.1, 20.0);
    scene.add_object(SceneObject::new(
        cube_mesh.clone(),
        floor_transform,
        0xFF_55_AA_55,
    ));

    // Add multiple cubes at different depths
    for i in 0..10 {
        let z = -(i as f32) * 3.0 - 5.0; // Distribute along -Z
        let x = if i % 2 == 0 { -2.0 } else { 2.0 };

        let color = if i % 3 == 0 {
            0xFF_FF_55_55
        } else if i % 3 == 1 {
            0xFF_55_55_FF
        } else {
            0xFF_FF_FF_55
        };

        scene.add_object(SceneObject::new(
            cube_mesh.clone(),
            Mat4::translation(x, 0.0, z),
            color,
        ));
    }

    while window.is_open() {
        let events = window.poll_events();
        for event in events {
            if matches!(event, Event::Close) {
                return;
            }
        }

        angle += 0.02;

        // Move camera forward and backward slowly
        let cam_z = (angle * 0.5).sin() * 5.0 + 5.0;
        let eye = Vec3::new(0.0, 2.0, cam_z);
        let target = Vec3::new(0.0, 0.0, -10.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at(eye, target, up);
        scene.camera.view = view;

        // Render scene
        fb.clear(0xFF_00_00_00); // Clear to black
        zb.clear();

        let mut renderer = abrash::rasterizer::tile::TileRenderer::new(width, height);
        scene.render(&mut renderer, &mut fb, &mut zb);

        // Apply Fog
        // Modulate fog color slightly to show dynamic changes
        let fog_r = ((angle.sin() * 0.5 + 0.5) * 50.0 + 50.0) as u32;
        let fog_g = ((angle.cos() * 0.5 + 0.5) * 50.0 + 50.0) as u32;
        let fog_b = 100;

        let fog_color = 0xFF_00_00_00 | (fog_r << 16) | (fog_g << 8) | fog_b;

        let adjusted_config = FogConfig {
            color: fog_color,
            near: 0.95,
            far: 1.0,
        };

        apply_fog(&mut fb, &zb, &adjusted_config);

        window.blit_framebuffer(&fb);
    }
}
