use engine::graphics::{
	flags, pipeline,
	types::{Mat4, Vec3, Vec4},
	vertex_object,
};
use nalgebra::{Point3, Vector3, Vector4};

pub struct Segment {
	pub pos1: Point3<f32>,
	pub pos2: Point3<f32>,
	pub color: Vector4<f32>,
	pub flags: Vector4<f32>,
}
impl From<((f32, f32, f32), (f32, f32, f32))> for Segment {
	fn from(params: ((f32, f32, f32), (f32, f32, f32))) -> Self {
		Self {
			pos1: Point3::new(params.0 .0, params.0 .1, params.0 .2),
			pos2: Point3::new(params.1 .0, params.1 .1, params.1 .2),
			color: Vector4::new(1.0, 1.0, 1.0, 1.0),
			flags: Vector4::default(),
		}
	}
}
impl From<(Vector3<f32>, Vector3<f32>)> for Segment {
	fn from(params: (Vector3<f32>, Vector3<f32>)) -> Self {
		Self {
			pos1: params.0.into(),
			pos2: params.1.into(),
			color: Vector4::new(1.0, 1.0, 1.0, 1.0),
			flags: Vector4::default(),
		}
	}
}
impl From<((f32, f32, f32), (f32, f32, f32), Vector4<f32>)> for Segment {
	fn from(params: ((f32, f32, f32), (f32, f32, f32), Vector4<f32>)) -> Self {
		Self {
			pos1: Point3::new(params.0 .0, params.0 .1, params.0 .2),
			pos2: Point3::new(params.1 .0, params.1 .1, params.1 .2),
			color: params.2,
			flags: Vector4::default(),
		}
	}
}
impl From<((f32, f32, f32), (f32, f32, f32), Vector4<f32>, Vector4<f32>)> for Segment {
	fn from(params: ((f32, f32, f32), (f32, f32, f32), Vector4<f32>, Vector4<f32>)) -> Self {
		Self {
			pos1: Point3::new(params.0 .0, params.0 .1, params.0 .2),
			pos2: Point3::new(params.1 .0, params.1 .1, params.1 .2),
			color: params.2,
			flags: params.3,
		}
	}
}

pub struct Segments(Vec<Segment>);
impl Segments {
	pub fn new() -> Self {
		Self(Vec::new())
	}

	pub fn push(&mut self, segment: Segment) {
		self.0.push(segment);
	}

	pub fn into_vertices(self) -> (Vec<Vertex>, Vec<u32>) {
		let mut vertices = Vec::new();
		let mut indices = Vec::new();
		for segment in self.0.into_iter() {
			for pos in [segment.pos1, segment.pos2].iter() {
				let i = vertices.len() as u32;
				vertices.push(Vertex {
					position: (*pos).into(),
					color: segment.color.into(),
					flags: segment.flags.into(),
				});
				indices.push(i);
			}
		}
		(vertices, indices)
	}

	pub fn with_color(mut self, color: Vector4<f32>) -> Self {
		for segment in self.0.iter_mut() {
			segment.color = color;
		}
		self
	}
}

#[vertex_object]
#[derive(Debug, Default, Clone)]
pub struct Vertex {
	#[vertex_attribute([R, G, B], Bit32, SFloat)]
	pub position: Vec3,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	pub color: Vec4,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	pub flags: Vec4,
}

#[vertex_object]
#[derive(Clone, Debug, Default)]
pub struct Instance {
	#[vertex_attribute([R, G, B], Bit32, SFloat)]
	pub chunk_coordinate: Vec3,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	#[vertex_span(4)]
	pub model_matrix: Mat4,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	pub color: Vec4,
}

pub struct BufferDrawSubset {
	pub model: ModelSubset,
	pub instance: InstanceSubset,
}
pub struct ModelSubset {
	pub index_start: usize,
	pub index_count: usize,
	pub vertex_start: usize,
}
pub struct InstanceSubset {
	pub start: usize,
	pub count: usize,
}
