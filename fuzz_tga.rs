use abrash::framebuffer::Framebuffer;
use arbitrary::{Arbitrary, Unstructured};

fn main() {
    let mut u = Unstructured::new(&[0; 1024]);
    let width = u.arbitrary::<u32>().unwrap();
    let height = u.arbitrary::<u32>().unwrap();

    if let Ok(mut fb) = Framebuffer::new(width, height) {
        let _ = fb.export_tga("test.tga");
    }
}
