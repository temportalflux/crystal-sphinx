use super::super::Face;
use crate::block;
use engine::{
	graphics::{
		flags, pipeline,
		types::{Mat4, Vec3, Vec4},
		vertex_object,
	},
	math::nalgebra::Translation3,
};
use enumset::EnumSet;

#[vertex_object]
#[derive(Clone, Debug, Default)]
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
	pub fn from(point: &block::Point, faces: EnumSet<Face>) -> Self {
		let flags = super::Flags { faces };
		Self {
			chunk_coordinate: point.chunk().coords.cast::<f32>().into(),
			model_matrix: Translation3::from(point.offset().coords.cast::<f32>())
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
