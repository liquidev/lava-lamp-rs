use std::ops::Range;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::shape::*;

#[derive(Copy, Clone, Debug)]
struct Blob {
  pos: (f32, f32),
  vel: (f32, f32),
  radius: f32,
}

impl Blob {
  pub fn new(pos: (f32, f32), vel: (f32, f32), radius: f32) -> Self {
    Blob { pos, vel, radius }
  }

  pub fn update(&mut self) {
    self.pos = (self.pos.0 + self.vel.0, self.pos.1 + self.vel.1);
  }

  pub fn add_to_shape(&self, shape: &mut Shape, step: f32) {
    let pos = (
      self.pos.0 + self.vel.0 * step,
      self.pos.1 + self.vel.1 * step,
    );
    shape.add_rect(
      (pos.0 - self.radius, pos.1 - self.radius),
      (self.radius * 2.0, self.radius * 2.0),
    );
  }
}

pub struct LavaLampConfig {
  pub spawn_chance: f32,
  pub blob_speed: Range<f32>,
  pub blob_size: Range<f32>,
}

pub struct LavaLamp {
  config: LavaLampConfig,

  blobs: Vec<Blob>,
  rng: SmallRng,
}

impl LavaLamp {
  pub fn new(config: LavaLampConfig) -> Self {
    Self {
      config,
      blobs: Vec::new(),
      rng: SmallRng::from_entropy(),
    }
  }

  fn spawn_blob<F>(&mut self, frame: &F)
  where
    F: glium::Surface,
  {
    let (screen_width, screen_height) = frame.get_dimensions();
    let pos = (
      self.rng.gen_range(0.0, screen_width as f32),
      screen_height as f32 + self.config.blob_size.end,
    );
    let vel = (
      0.0,
      -self
        .rng
        .gen_range(self.config.blob_speed.start, self.config.blob_speed.end),
    );
    let radius = self
      .rng
      .gen_range(self.config.blob_size.start, self.config.blob_size.end)
      / 2.0;
    let blob = Blob::new(pos, vel, radius);
    self.blobs.push(blob)
  }

  fn collect_garbage(&mut self) {
    let mut garbage_indices: Vec<usize> = Vec::new();
    for (index, blob) in self.blobs.iter().enumerate() {
      if blob.pos.1 <= -self.config.blob_size.end {
        garbage_indices.push(index);
      }
    }
    for &index in garbage_indices.iter().rev() {
      self.blobs.remove(index);
    }
  }

  pub fn update<F>(&mut self, frame: &F)
  where
    F: glium::Surface,
  {
    for blob in &mut self.blobs {
      blob.update();
    }
    if self.rng.gen::<f32>() < self.config.spawn_chance {
      self.spawn_blob(frame);
    }
    self.collect_garbage();
  }

  pub fn add_to_shape(&self, shape: &mut Shape, step: f32) {
    for blob in &self.blobs {
      blob.add_to_shape(shape, step);
    }
  }
}
