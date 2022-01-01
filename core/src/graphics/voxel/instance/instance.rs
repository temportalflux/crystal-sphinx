use super::super::Face;
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
	pub fn from(chunk: &Point3<i64>, offset: &Point3<i8>) -> Self {
		let flags = super::Flags {
			faces: EnumSet::all(),
		};
		Self {
			chunk_coordinate: Vector3::new(chunk.x as f32, chunk.y as f32, chunk.z as f32).into(),
			model_matrix: Translation3::new(offset.x as f32, offset.y as f32, offset.z as f32)
				.to_homogeneous()
				.into(),
			instance_flags: flags.build().into(),
		}
	}

	pub fn faces(&self) -> EnumSet<Face> {
		let faces_enabled_bitfield: u32 = unsafe { std::mem::transmute(self.instance_flags[0]) };
		Face::parse_model_bit(faces_enabled_bitfield)
	}

	pub fn set_faces(&mut self, faces: EnumSet<Face>) {
		let mut flags: super::Flags = (*self.instance_flags).into();
		flags.faces = faces;
		self.instance_flags = flags.build().into();
	}
}
