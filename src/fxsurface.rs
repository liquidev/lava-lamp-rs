use std::collections::HashMap;
use std::error::Error;

use glium::program::ProgramCreationError;
use glium::texture::TextureCreationError;

use crate::shape::*;

#[derive(Clone)]
pub struct UniformTable<'a> {
  uniforms: HashMap<String, glium::uniforms::UniformValue<'a>>,
}

impl<'a> UniformTable<'a> {
  pub fn new() -> Self {
    Self {
      uniforms: HashMap::new(),
    }
  }

  pub fn add<T>(&mut self, name: &str, val: &'a T) -> &mut Self
    where T: glium::uniforms::AsUniformValue,
  {
    self.uniforms.insert(name.to_string(), val.as_uniform_value());
    self
  }
}

impl<'a> glium::uniforms::Uniforms for UniformTable<'a> {
  fn visit_values<'b, F>(&'b self, mut callback: F)
    where F: FnMut(&str, glium::uniforms::UniformValue<'b>)
  {
    for (key, val) in &self.uniforms {
      callback(key.as_str(), val.clone());
    }
  }
}

pub struct FxBuffer {
  first: glium::Texture2d,
  second: glium::Texture2d,
}

impl FxBuffer {
  pub fn new(
    display: &glium::Display,
    size: (u32, u32),
  ) -> Result<Self, TextureCreationError> {
    Ok(FxBuffer {
      first: glium::Texture2d::empty(display, size.0, size.1)?,
      second: glium::Texture2d::empty(display, size.0, size.1)?,
    })
  }

  pub fn resize(
    &mut self,
    display: &glium::Display,
    size: (u32, u32),
  ) -> Result<(), TextureCreationError> {
    self.first = glium::Texture2d::empty(display, size.0, size.1)?;
    self.second = glium::Texture2d::empty(display, size.0, size.1)?;
    Ok(())
  }

  pub fn draw_to<F>(&mut self, mut callback: F)
    where F: FnMut(&mut glium::Texture2d),
  {
    callback(&mut self.first);
  }

  pub fn effect<'n>(
    &mut self,
    shape_buffer: &mut ShapeBuffer,
    program: &glium::Program,
    uniforms: &UniformTable,
  ) -> Result<(), Box<dyn Error>> {
    use glium::Surface;
    {
      let mut second_surface = self.second.as_surface();
      second_surface.clear_color(0.0, 0.0, 0.0, 0.0);

      let mut first_rect = Shape::new();
      first_rect.add_rect((-1.0, 1.0), (2.0, -2.0));
      let blending_fn = glium::BlendingFunction::Addition {
        source: glium::LinearBlendingFactor::One,
        destination: glium::LinearBlendingFactor::OneMinusSourceAlpha,
      };
      let mut new_uniforms = uniforms.clone();
      let first_sampler = self.first.sampled();
      new_uniforms.add("source_texture", &first_sampler);
      shape_buffer.draw(
        &mut second_surface,
        &first_rect,
        program,
        &new_uniforms,
        &glium::DrawParameters {
          blend: glium::Blend {
            color: blending_fn,
            alpha: blending_fn,
            ..Default::default()
          },
          ..Default::default()
        },
      )?;
    }
    std::mem::swap(&mut self.first, &mut self.second);
    Ok(())
  }
}

const VSH_EFFECT: &str = r#"
  #version 330 core

  in vec2 pos;
  in vec2 texture_uv;
  out vec2 in_uv;

  void main() {
    gl_Position = vec4(pos.x, pos.y, 0.0, 1.0);
    in_uv = texture_uv;
  }
"#;

pub fn create_effect(
  display: &glium::Display,
  src: &str,
) -> Result<glium::Program, ProgramCreationError> {
  glium::Program::from_source(display, VSH_EFFECT, src, None)
}

