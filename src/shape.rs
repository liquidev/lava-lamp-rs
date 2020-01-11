use std::error::Error;

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
  pub pos: [f32; 2],
  pub texture_uv: [f32; 2],
}
implement_vertex!(Vertex, pos, texture_uv);

impl Vertex {
  pub fn textured(x: f32, y: f32, u: f32, v: f32) -> Self {
    Self {
      pos: [x, y],
      texture_uv: [u, v],
    }
  }

  pub fn plain(x: f32, y: f32) -> Self {
    Self::textured(x, y, 0.0, 0.0)
  }
}

#[derive(Clone)]
pub struct Shape {
  pub vertices: Vec<Vertex>,
}

impl Shape {
  pub fn new() -> Self {
    Self {
      vertices: Vec::new(),
    }
  }

  pub fn clear(&mut self) {
    self.vertices.clear();
  }

  pub fn add_vert(&mut self, vertex: Vertex) {
    self.vertices.push(vertex);
  }

  pub fn add_uv_rect(
    &mut self,
    pos: (f32, f32),
    size: (f32, f32),
    uv_rect: (f32, f32, f32, f32),
  ) {
    let uv = (uv_rect.0, 1.0 - uv_rect.1, uv_rect.2, -uv_rect.3);
    self.add_vert(Vertex::textured(pos.0, pos.1, uv.0, uv.1));
    self.add_vert(Vertex::textured(pos.0 + size.0, pos.1, uv.0 + uv.2, uv.1));
    self.add_vert(Vertex::textured(pos.0, pos.1 + size.1, uv.0, uv.1 + uv.3));
    self.add_vert(Vertex::textured(pos.0 + size.0, pos.1, uv.0 + uv.2, uv.1));
    self.add_vert(Vertex::textured(
      pos.0 + size.0,
      pos.1 + size.1,
      uv.0 + uv.2,
      uv.1 + uv.3,
    ));
    self.add_vert(Vertex::textured(pos.0, pos.1 + size.1, uv.0, uv.1 + uv.3));
  }

  pub fn add_rect(&mut self, pos: (f32, f32), size: (f32, f32)) {
    self.add_uv_rect(pos, size, (0.0, 0.0, 1.0, 1.0))
  }
}

pub struct ShapeBuffer<'s> {
  display: &'s glium::Display,
  current_size: usize,
  vbo: glium::VertexBuffer<Vertex>,
  ebo: glium::index::NoIndices,
}

const DEFAULT_SHAPEBUFFER_SIZE: usize = 32;

impl<'s> ShapeBuffer<'s> {
  fn reallocate(&mut self) -> Result<(), glium::vertex::BufferCreationError> {
    self.vbo =
      glium::VertexBuffer::empty_dynamic(self.display, self.current_size)?;
    Ok(())
  }

  fn update(
    &mut self,
    shape: &Shape,
  ) -> Result<(), glium::vertex::BufferCreationError> {
    if shape.vertices.len() > self.current_size {
      self.current_size = shape.vertices.len();
      self.reallocate()?;
    }
    let mut padded = shape.vertices.clone();
    padded.resize(self.current_size, Vertex::plain(0.0, 0.0));
    self.vbo.write(&padded);
    Ok(())
  }

  pub fn new(
    display: &'s glium::Display,
  ) -> Result<Self, glium::vertex::BufferCreationError> {
    Ok(Self {
      display,
      current_size: DEFAULT_SHAPEBUFFER_SIZE,
      vbo: glium::VertexBuffer::empty_dynamic(
        display,
        DEFAULT_SHAPEBUFFER_SIZE,
      )?,
      ebo: glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
    })
  }

  pub fn draw<T, U>(
    &mut self,
    target: &mut T,
    shape: &Shape,
    program: &glium::Program,
    uniforms: &U,
    params: &glium::DrawParameters,
  ) -> Result<(), Box<dyn Error>>
  where
    T: glium::Surface,
    U: glium::uniforms::Uniforms,
  {
    self.update(shape)?;
    target.draw(
      self.vbo.slice(0..shape.vertices.len()).unwrap(),
      &self.ebo,
      program,
      uniforms,
      params,
    )?;
    Ok(())
  }
}
