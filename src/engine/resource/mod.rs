use std::ops::Deref;

use egui::ColorImage;

pub use manager::*;
pub use progress::*;

pub mod progress;
pub mod manager;

//
// #[repr(transparent)]
// #[derive(Clone)]
// pub struct FontWrapper(pub FontArc);
//
// impl Deref for FontWrapper {
//     type Target = FontArc;
//
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }
//
// impl From<&FontArc> for FontWrapper {
//     fn from(f: &FontArc) -> Self {
//         Self {
//             0: f.clone()
//         }
//     }
// }

#[allow(unused)]
pub fn load_image_from_memory(image_data: &[u8]) -> Result<ColorImage, image::ImageError> {
    let image = image::load_from_memory(image_data)?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
}
