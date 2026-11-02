//! Framebuffer + "G-buffer": durante la rasterización solo guardamos el
//! fragmento más cercano de cada píxel (early depth test). Después el fragment
//! shader se ejecuta UNA vez por píxel visible, en paralelo con rayon.

use nalgebra_glm::Vec3;

#[derive(Clone, Copy)]
pub struct Fragment {
    pub depth: f32,
    pub world: Vec3,
    pub obj: Vec3,
    /// Índice de la instancia (cuerpo celeste) dueña del fragmento.
    pub instance: u16,
}

pub const NO_INSTANCE: u16 = u16::MAX;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub color: Vec<u32>,
    pub fragments: Vec<Fragment>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            color: vec![0; width * height],
            fragments: vec![Self::empty(); width * height],
        }
    }

    fn empty() -> Fragment {
        Fragment {
            depth: f32::INFINITY,
            world: Vec3::zeros(),
            obj: Vec3::zeros(),
            instance: NO_INSTANCE,
        }
    }

    pub fn clear(&mut self) {
        let e = Self::empty();
        self.fragments.iter_mut().for_each(|f| *f = e);
    }

    pub fn save_png(&self, path: &str) {
        let mut img = image::RgbImage::new(self.width as u32, self.height as u32);
        for (i, c) in self.color.iter().enumerate() {
            let x = (i % self.width) as u32;
            let y = (i / self.width) as u32;
            img.put_pixel(
                x,
                y,
                image::Rgb([(c >> 16) as u8, (c >> 8) as u8, *c as u8]),
            );
        }
        img.save(path).expect("no se pudo guardar el PNG");
    }
}

pub fn to_u32(c: Vec3) -> u32 {
    let r = (c.x.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (c.y.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (c.z.clamp(0.0, 1.0) * 255.0) as u32;
    (r << 16) | (g << 8) | b
}
