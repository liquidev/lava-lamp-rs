// kudos to Nim for type propagation
// for some reason Rust has been out there for so long, but it still makes you write types like a C caveman
fn gaussian_2d(
  x: f32,
  y: f32,
  a: f32,
  ox: f32,
  oy: f32,
  sx: f32,
  sy: f32,
) -> f32 {
  a * (-((x - ox).powi(2) / (2.0 * sx.powi(2))
    + (y - oy).powi(2) / (2.0 * sy.powi(2))))
  .exp()
}

pub fn gen_blob_gaussian(
  display: &glium::Display,
  size: (u32, u32),
) -> Result<glium::Texture2d, glium::texture::TextureCreationError> {
  use glium::texture::{RawImage2d, Texture2d};

  let mut data: Vec<f32> = Vec::new();

  for py in 0..size.1 {
    for px in 0..size.0 {
      let fx = (px as f32) / (size.0 as f32) * 2.0 - 1.0;
      let fy = (py as f32) / (size.1 as f32) * 2.0 - 1.0;
      let alpha = gaussian_2d(fx, fy, 1.0, 0.0, 0.0, 0.25, 0.25);
      data.extend_from_slice(&[1.0, 1.0, 1.0, alpha]);
    }
  }

  let image = RawImage2d::from_raw_rgba(data, size);

  Texture2d::new(display, image)
}
