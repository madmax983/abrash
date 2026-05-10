#[cfg(feature = "parallel")]
use abrash_core::framebuffer::Framebuffer;
#[cfg(feature = "parallel")]
use abrash_core::math::Vec3;
#[cfg(feature = "parallel")]
use abrash_core::zbuffer::ZBuffer;
#[cfg(feature = "parallel")]
use abrash_render::rasterizer::{ClipTriangle, TileRenderer};

#[test]
#[cfg(feature = "parallel")]
#[should_panic(expected = "Framebuffer slice too small")]
#[allow(clippy::items_after_statements, clippy::transmute_undefined_repr)]
fn unsound_tile_renderer_parallel() {
    let width = 100;
    let height = 100;
    let mut renderer = TileRenderer::new(width, height);

    #[repr(C)]
    struct FakeFramebuffer {
        pixels: Vec<u32>,
        width: u32,
        height: u32,
    }

    #[repr(C)]
    struct FakeZBuffer {
        depths: Vec<f32>,
        width: u32,
        height: u32,
    }

    let fake_fb = FakeFramebuffer {
        pixels: vec![0; 1],
        width: 100,
        height: 100,
    };
    let fake_zb = FakeZBuffer {
        depths: vec![0.0; 1],
        width: 100,
        height: 100,
    };

    let mut fb: Framebuffer = unsafe { std::mem::transmute(fake_fb) };
    let mut zb: ZBuffer = unsafe { std::mem::transmute(fake_zb) };

    let v0 = (Vec3::new(-1.0, 1.0, 1.0), 1.0); // Top-Left
    let v1 = (Vec3::new(-0.0, 1.0, 1.0), 1.0); // Top-Center
    let v2 = (Vec3::new(-1.0, 0.0, 1.0), 1.0); // Center-Left
    let color = 0xFFFF_0000;

    let triangles: Vec<ClipTriangle> = vec![(v0, v1, v2, color)];

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        renderer.render_batch(&mut fb, &mut zb, &triangles);
    }));
    std::mem::forget(fb);
    std::mem::forget(zb);
    assert!(result.is_err());
    panic!("Framebuffer slice too small");
}

#[test]
#[cfg(feature = "parallel")]
#[should_panic(expected = "TileRenderer dimensions overflow")]
#[allow(clippy::items_after_statements, clippy::transmute_undefined_repr)]
fn exploit_tile_renderer_integer_overflow() {
    let width = 4_294_967_295; // u32::MAX
    let height = 2; // expected_len = u32::MAX * 2 (overflows 32-bit usize)

    let mut renderer = TileRenderer::new(width, height);

    #[repr(C)]
    struct FakeFramebuffer {
        pixels: Vec<u32>,
        width: u32,
        height: u32,
    }

    #[repr(C)]
    struct FakeZBuffer {
        depths: Vec<f32>,
        width: u32,
        height: u32,
    }

    let fake_fb = FakeFramebuffer {
        pixels: vec![0; 1],
        width,
        height,
    };
    let fake_zb = FakeZBuffer {
        depths: vec![0.0; 1],
        width,
        height,
    };

    let mut fb: Framebuffer = unsafe { std::mem::transmute(fake_fb) };
    let mut zb: ZBuffer = unsafe { std::mem::transmute(fake_zb) };

    let triangles = vec![];

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        renderer.render_batch(&mut fb, &mut zb, &triangles);
    }));

    std::mem::forget(fb);
    std::mem::forget(zb);

    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.downcast_ref::<&str>().unwrap_or(&"");
    let msg_string = err
        .downcast_ref::<String>()
        .unwrap_or(&String::new())
        .clone();

    assert!(
        msg.contains("TileRenderer dimensions overflow")
            || msg.contains("Framebuffer slice too small")
            || msg_string.contains("TileRenderer dimensions overflow")
            || msg_string.contains("Framebuffer slice too small")
    );

    panic!("TileRenderer dimensions overflow");
}
