use abrash_core::framebuffer::Framebuffer;
use fastrand::Rng;

const FIRE_PALETTE: [u32; 36] = [
    0xFF070707, 0xFF1F0707, 0xFF2F0F07, 0xFF470F07,
    0xFF571707, 0xFF671F07, 0xFF771F07, 0xFF8F2707,
    0xFF9F2F07, 0xFFAF3F07, 0xFFBF4707, 0xFFC74707,
    0xFFDF4F07, 0xFFDF5707, 0xFFDF5707, 0xFFD75F07,
    0xFFD7670F, 0xFFCF6F0F, 0xFFCF770F, 0xFFCF7F0F,
    0xFFCF8717, 0xFFC78717, 0xFFC78F17, 0xFFC7971F,
    0xFFBF9F1F, 0xFFBF9F1F, 0xFFBFA727, 0xFFBFA727,
    0xFFBFB72F, 0xFFB7B72F, 0xFFB7BF2F, 0xFFB7C737,
    0xFFCFCF6F, 0xFFDFDF9F, 0xFFEFEFC7, 0xFFFFFFFF,
];

pub struct FireEffect {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    rng: Rng,
}

impl FireEffect {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height) as usize],
            rng: Rng::new(),
        }
    }

    pub fn buffer(&self) -> &[u8] {
        &self.pixels
    }

    pub fn seed_bottom_row(&mut self) {
        let y = self.height - 1;
        for x in 0..self.width {
            let index = (y * self.width + x) as usize;
            self.pixels[index] = 35;
        }
    }

    pub fn update(&mut self) {
        // Calculate new fire pixel values from bottom to top
        for y in (1..self.height).rev() {
            for x in 0..self.width {
                let src_idx = (y * self.width + x) as usize;
                let pixel = self.pixels[src_idx];

                if pixel == 0 {
                    let dst_idx = ((y - 1) * self.width + x) as usize;
                    self.pixels[dst_idx] = 0;
                } else {
                    let rand = (self.rng.u8(..) & 3) as u32;
                    let new_x = x.saturating_sub(rand & 1);
                    // Prevent overflow on new_x if subtracting
                    let dst_x = if new_x >= self.width { self.width - 1 } else { new_x };
                    let dst_y = y - 1;
                    let dst_idx = (dst_y * self.width + dst_x) as usize;

                    let new_pixel = pixel.saturating_sub((rand & 1) as u8);
                    self.pixels[dst_idx] = new_pixel;
                }
            }
        }
    }
}

pub fn render_fire(fb: &mut Framebuffer, fire: &FireEffect) {
    let width = fb.width().min(fire.width);
    let height = fb.height().min(fire.height);

    for y in 0..height {
        let y_offset = (y * fire.width) as usize;
        for x in 0..width {
            let idx = y_offset + x as usize;
            let val = fire.pixels[idx] as usize;
            let color = FIRE_PALETTE[val.min(35)];
            // bounds are already checked by min(fb.width/height)
            unsafe {
                fb.set_pixel_unchecked(x as usize, y as usize, color);
            }
        }
    }
}
