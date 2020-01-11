#[macro_use]
extern crate glium;

mod blob;
mod fxsurface;
mod lamp;
mod shape;

use std::error::Error;
use std::time::SystemTime;

use structopt::StructOpt;

use blob::*;
use fxsurface::*;
use lamp::*;
use shape::*;

const VSH_DEFAULT: &str = r#"
  #version 330 core

  uniform mat4 projection;

  in vec2 pos;
  in vec2 texture_uv;
  out vec2 in_uv;

  void main() {
    gl_Position = vec4(pos.x, pos.y, 0.0, 1.0) * projection;
    in_uv = texture_uv;
  }
"#;

const FSH_DEFAULT: &str = r#"
  #version 330 core

  uniform sampler2D tex;

  in vec2 in_uv;
  out vec4 out_color;

  void main() {
    out_color = texture(tex, in_uv);
  }
"#;

const FX_ALPHA_THRESHOLD: &str = r#"
  #version 330 core

  uniform float threshold;
  uniform float smoothness;

  uniform sampler2D source_texture;

  in vec2 in_uv;
  out vec4 out_color;

  void main() {
    vec4 col = texture(source_texture, in_uv);
    col.a =
      smoothstep(threshold - smoothness, threshold + smoothness, col.a);

    out_color = col;
  }
"#;

const FX_TINT: &str = r#"
  #version 330 core

  uniform vec3 tint;

  uniform sampler2D source_texture;

  in vec2 in_uv;
  out vec4 out_color;

  void main() {
    out_color = texture(source_texture, in_uv) * vec4(tint, 1.0);
  }
"#;

const UPDATE_FREQUENCY: u32 = 60;
const SECONDS_PER_UPDATE: f64 = 1.0 / (UPDATE_FREQUENCY as f64);

#[derive(Copy, Clone)]
struct ColorRgb(f32, f32, f32);

impl ColorRgb {
  fn linear(&self) -> Self {
    let source = self.clone();
    Self(
      source.0.powf(2.2),
      source.1.powf(2.2),
      source.2.powf(2.2),
    )
  }
}

impl std::str::FromStr for ColorRgb {
  type Err = std::num::ParseIntError;

  fn from_str(col_str: &str) -> Result<Self, Self::Err> {
    let raw_col = u32::from_str_radix(col_str, 16)?;
    let red_u = (raw_col & 0x00ff0000) >> 16;
    let green_u = (raw_col & 0x0000ff00) >> 8;
    let blue_u = raw_col & 0x000000ff;
    Ok(ColorRgb(
      red_u as f32 / 256.0,
      green_u as f32 / 256.0,
      blue_u as f32 / 256.0,
    ).linear())
  }
}

impl glium::uniforms::AsUniformValue for ColorRgb {
  fn as_uniform_value(&self) -> glium::uniforms::UniformValue {
    glium::uniforms::UniformValue::Vec3([self.0, self.1, self.2])
  }
}

struct F32Range {
  start: f32,
  end: f32,
}

#[derive(Debug)]
enum RangeParseError {
  MissingLeft,
  MissingRight,
  MissingDelimiter,
  InvalidFloat(std::num::ParseFloatError),
}

impl RangeParseError {
  pub fn to_string(&self) -> String {
    match self {
      RangeParseError::MissingLeft => {
        String::from("missing left-hand side of range")
      }
      RangeParseError::MissingRight => {
        String::from("missing right-hand side of range")
      }
      RangeParseError::MissingDelimiter => {
        String::from("missing '..' delimiter")
      }
      RangeParseError::InvalidFloat(err) => {
        String::from(format!("invalid float: {}", err.to_string()))
      }
    }
  }
}

impl std::str::FromStr for F32Range {
  type Err = RangeParseError;

  fn from_str(range_str: &str) -> Result<Self, Self::Err> {
    if let Some(delimiter_index) = range_str.find("..") {
      let left_s = range_str.get(..delimiter_index);
      let right_s = range_str.get((delimiter_index + 2)..);
      if left_s.is_none() {
        Err(RangeParseError::MissingLeft)
      } else if right_s.is_none() {
        Err(RangeParseError::MissingRight)
      } else {
        let left_f = f32::from_str(left_s.unwrap());
        let right_f = f32::from_str(right_s.unwrap());
        if left_f.is_err() {
          Err(RangeParseError::InvalidFloat(left_f.err().unwrap()))
        } else if right_f.is_err() {
          Err(RangeParseError::InvalidFloat(right_f.err().unwrap()))
        } else {
          Ok(F32Range {
            start: left_f.unwrap(),
            end: right_f.unwrap(),
          })
        }
      }
    } else {
      Err(RangeParseError::MissingDelimiter)
    }
  }
}

#[derive(StructOpt)]
#[structopt(name = "blobs")]
struct CliParams {
  #[structopt(short, default_value = "db5461")]
  background_color: ColorRgb,

  #[structopt(short, default_value = "e57369")]
  foreground_color: ColorRgb,

  #[structopt(long, default_value = "0.4")]
  threshold: f32,

  #[structopt(long, default_value = "0.025")]
  smooth: f32,

  #[structopt(short = "c", long, default_value = "0.02")]
  spawn_chance: f32,

  #[structopt(short = "s", long = "speed", default_value = "0.5..1.0")]
  blob_speed: F32Range,

  #[structopt(short = "S", long = "size", default_value = "32.0..128.0")]
  blob_size: F32Range,
}

#[rustfmt::skip]
fn ortho(
  top: f32, bottom: f32,
  left: f32, right: f32,
  far: f32, near: f32,
) -> [[f32; 4]; 4] {
  [[2.0 / (right - left), 0.0, 0.0, -((right + left) / (right - left))],
   [0.0, 2.0 / (top - bottom), 0.0, -((top + bottom) / (top - bottom))],
   [0.0, 0.0, -2.0 / (far - near), -((far + near) / (far - near))],
   [0.0, 0.0, 0.0, 1.0]]
}

fn screen_projection(size: (u32, u32)) -> [[f32; 4]; 4] {
  let (width, height) = size;
  ortho(0.0, height as f32, 0.0, width as f32, 1.0, -1.0)
}

fn time_in_seconds(start: SystemTime) -> f64 {
  let now = SystemTime::now();
  let time = now.duration_since(start).unwrap();
  time.as_secs_f64()
}

fn main() -> Result<(), Box<dyn Error>> {
  use glium::glutin;
  use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter};
  use glium::Surface;

  // CLI
  let config = CliParams::from_args();
  let lamp_config = LavaLampConfig {
    spawn_chance: config.spawn_chance,
    blob_size: config.blob_size.start..config.blob_size.end,
    blob_speed: config.blob_speed.start..config.blob_speed.end,
  };

  // timing stuff
  let start_time = SystemTime::now();

  // window
  let mut event_loop = glutin::EventsLoop::new();
  let wb = glutin::WindowBuilder::new();
  let cb = glutin::ContextBuilder::new();
  let display = glium::Display::new(wb, cb, &event_loop)?;

  // resources
  let blob_texture = gen_blob_gaussian(&display, (64, 64))?;
  let mut shape = Shape::new();
  let mut shape_buffer = ShapeBuffer::new(&display)?;
  let prog_default =
    glium::Program::from_source(&display, VSH_DEFAULT, FSH_DEFAULT, None)?;
  let prog_fx_alpha_threshold = create_effect(&display, FX_ALPHA_THRESHOLD)?;
  let prog_fx_tint = create_effect(&display, FX_TINT)?;

  let window_size = display.gl_window().window().get_inner_size().unwrap();
  let mut fx_buffer = FxBuffer::new(
    &display,
    (window_size.width.round() as u32, window_size.height.round() as u32),
  )?;

  // drawing
  let blending_fn = glium::BlendingFunction::Addition {
    source: glium::LinearBlendingFactor::SourceAlpha,
    destination: glium::LinearBlendingFactor::OneMinusSourceAlpha,
  };
  let draw_params = glium::DrawParameters {
    blend: glium::Blend {
      color: blending_fn,
      alpha: blending_fn,
      ..Default::default()
    },
    ..Default::default()
  };

  // state
  let mut lava_lamp = LavaLamp::new(lamp_config);

  // timings
  let mut previous_time = time_in_seconds(start_time);
  let mut lag = 0.0f64;

  let mut closed = false;
  while !closed {
    // measure time
    let current_time = time_in_seconds(start_time);
    let elapsed_time = current_time - previous_time;
    previous_time = current_time;
    lag += elapsed_time;

    // poll the events
    event_loop.poll_events(|ev| match ev {
      glutin::Event::WindowEvent { event, .. } => match event {
        glutin::WindowEvent::CloseRequested => closed = true,
        glutin::WindowEvent::Resized(size) => {
          fx_buffer.resize(
            &display,
            (size.width.round() as u32, size.height.round() as u32),
          )
          .unwrap();
        }
        _ => (),
      },
      _ => (),
    });

    // get the framebuffers, but don't draw yet. we only need them for screen
    // dimensions when spawning blobs
    let mut window_surface = display.draw();
    let mut blobs_size: (u32, u32) = (0, 0);

    fx_buffer.draw_to(|texture| {
      let mut blobs_surface = texture.as_surface();
      // update the lamp
      while lag >= SECONDS_PER_UPDATE {
        lava_lamp.update(&blobs_surface);
        lag -= SECONDS_PER_UPDATE;
      }

      // draw the lamp onto the effect surface
      blobs_size = blobs_surface.get_dimensions();

      blobs_surface.clear_color(1.0, 1.0, 1.0, 0.0);
      shape.clear();

      let step = lag / SECONDS_PER_UPDATE;
      lava_lamp.add_to_shape(&mut shape, step as f32);

      shape_buffer.draw(
        &mut blobs_surface,
        &shape,
        &prog_default,
        &uniform! {
          projection: screen_projection(blobs_size),
          tex: blob_texture.sampled()
            .minify_filter(MinifySamplerFilter::Linear)
            .magnify_filter(MagnifySamplerFilter::Linear),
        },
        &draw_params,
      ).unwrap();
    });

    fx_buffer.effect(
      &mut shape_buffer,
      &prog_fx_alpha_threshold,
      &UniformTable::new()
        .add("threshold", &config.threshold)
        .add("smoothness", &config.smooth),
    )?;
    fx_buffer.effect(
      &mut shape_buffer,
      &prog_fx_tint,
      &UniformTable::new()
        .add("tint", &config.foreground_color),
    )?;

    window_surface.clear_color(
      config.background_color.0,
      config.background_color.1,
      config.background_color.2,
      1.0,
    );

    // draw the effect surface back to the main window
    let (window_width, window_height) = window_surface.get_dimensions();
    fx_buffer.draw_to(|texture| {
      shape.clear();
      shape.add_rect((0.0, 0.0), (window_width as f32, window_height as f32));
      shape_buffer.draw(
        &mut window_surface,
        &shape,
        &prog_default,
        &uniform! {
          projection: screen_projection((window_width, window_height)),
          tex: texture.sampled(),
        },
        &draw_params,
      ).unwrap();
    });

    window_surface.finish()?;
  }

  Ok(())
}
