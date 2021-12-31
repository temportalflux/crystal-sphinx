use engine::{
	graphics::{
		flags, pipeline,
		types::{Mat4, Vec3, Vec4},
		vertex_object,
	},
	math::nalgebra::{Point3, Translation3, Vector3},
};
use enumset::EnumSet;

#[vertex_object]
#[derive(Debug, Default, Clone)]
pub struct Instance {
	#[vertex_attribute([R, G, B], Bit32, SFloat)]
	pub chunk_coordinate: Vec3,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	#[vertex_span(4)]
	pub model_matrix: Mat4,

	#[vertex_attribute([R, G, B, A], Bit32, SFloat)]
	pub instance_flags: Vec4,
}

impl Instance {
	pub fn from(chunk: &Point3<i64>, offset: &Point3<usize>) -> Self {
		Self {
			chunk_coordinate: Vector3::new(chunk.x as f32, chunk.y as f32, chunk.z as f32).into(),
			model_matrix: Translation3::new(offset.x as f32, offset.y as f32, offset.z as f32)
				.to_homogeneous()
				.into(),
			instance_flags: super::Flags {
				faces: EnumSet::all(),
			}
			.into(),
		}
	}
}
